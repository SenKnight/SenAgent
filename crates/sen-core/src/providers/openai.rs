//! OpenAI 兼容 Provider：同一实现支持两种线协议。
//!
//! - `WireApi::Chat`：`POST {base}/chat/completions`，消息数组 + delta 流
//! - `WireApi::Responses`：`POST {base}/responses`，input 项 + 类型化 SSE 事件流
//!
//! 两者的流式差异（delta 分片 vs 类型化事件、工具调用的累积方式）都在本模块
//! 内归一化为统一的 [`StreamEvent`]。

use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use async_trait::async_trait;
use futures_util::{Stream, StreamExt};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::config::{ProviderConfig, WireApi};
use crate::error::{Error, Result};
use crate::memory::{Message, Role};

use super::{sse_stream, ChatRequest, EventStream, Provider, SseEvent, StreamEvent, ToolSpec};

pub struct OpenAiProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
    headers: HashMap<String, String>,
    wire: WireApi,
    name: String,
    model: String,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
}

impl OpenAiProvider {
    pub fn new(cfg: &ProviderConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(30))
            .build()
            .map_err(Error::Http)?;
        Ok(Self {
            client,
            base_url: cfg.base_url.trim_end_matches('/').to_string(),
            api_key: cfg.resolved_api_key(),
            headers: cfg.headers.clone(),
            wire: cfg.wire_api,
            name: cfg.name.clone(),
            model: cfg.model.clone(),
            max_tokens: cfg.max_tokens,
            temperature: cfg.temperature,
        })
    }

    fn request(&self, path: &str) -> reqwest::RequestBuilder {
        let mut rb = self.client.post(format!("{}/{}", self.base_url, path));
        if let Some(key) = &self.api_key {
            rb = rb.bearer_auth(key);
        }
        for (k, v) in &self.headers {
            rb = rb.header(k.as_str(), v.as_str());
        }
        rb
    }

    /// chat 协议：/v1/chat/completions
    async fn chat_completions(&self, req: ChatRequest) -> Result<EventStream> {
        let mut messages: Vec<Value> = Vec::with_capacity(req.messages.len() + 1);
        if let Some(sys) = &req.system {
            messages.push(json!({"role": "system", "content": sys}));
        }
        for m in &req.messages {
            messages.push(chat_message(m));
        }

        let mut body = json!({
            "model": req.model,
            "messages": messages,
            "stream": true,
            "stream_options": {"include_usage": true}
        });
        if !req.tools.is_empty() {
            body["tools"] = Value::Array(
                req.tools
                    .iter()
                    .map(|t| {
                        json!({
                            "type": "function",
                            "function": {
                                "name": t.name,
                                "description": t.description,
                                "parameters": t.parameters
                            }
                        })
                    })
                    .collect(),
            );
        }
        if let Some(mt) = req.max_tokens.or(self.max_tokens) {
            body["max_tokens"] = json!(mt);
        }
        if let Some(t) = req.temperature.or(self.temperature) {
            body["temperature"] = json!(t);
        }

        let resp = self
            .request("chat/completions")
            .json(&body)
            .send()
            .await
            .map_err(Error::Http)?;
        let resp = check_response(resp).await?;
        Ok(Box::pin(normalize_chat_stream(sse_stream(resp.bytes_stream()))))
    }

    /// responses 协议：/v1/responses
    async fn responses(&self, req: ChatRequest) -> Result<EventStream> {
        let mut input: Vec<Value> = Vec::with_capacity(req.messages.len());
        for m in &req.messages {
            match m.role {
                Role::User => input.push(json!({
                    "type": "message",
                    "role": "user",
                    "content": [{"type": "input_text", "text": m.content}]
                })),
                Role::Assistant => {
                    if !m.content.is_empty() {
                        input.push(json!({
                            "type": "message",
                            "role": "assistant",
                            "content": [{"type": "output_text", "text": m.content}]
                        }));
                    }
                    if let Some(tcs) = &m.tool_calls {
                        for t in tcs {
                            input.push(json!({
                                "type": "function_call",
                                "call_id": t.id,
                                "name": t.name,
                                "arguments": t.arguments
                            }));
                        }
                    }
                }
                Role::Tool => input.push(json!({
                    "type": "function_call_output",
                    "call_id": m.tool_call_id.clone().unwrap_or_default(),
                    "output": m.content
                })),
                // system 通过 instructions 传递，不走 input
                Role::System => {}
            }
        }

        let mut body = json!({
            "model": req.model,
            "input": input,
            "stream": true,
            "store": false
        });
        if let Some(sys) = &req.system {
            body["instructions"] = json!(sys);
        }
        if !req.tools.is_empty() {
            body["tools"] = Value::Array(
                req.tools
                    .iter()
                    .map(|t| {
                        json!({
                            "type": "function",
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters
                        })
                    })
                    .collect(),
            );
        }
        if let Some(mt) = req.max_tokens.or(self.max_tokens) {
            body["max_output_tokens"] = json!(mt);
        }
        if let Some(t) = req.temperature.or(self.temperature) {
            body["temperature"] = json!(t);
        }

        let resp = self
            .request("responses")
            .json(&body)
            .send()
            .await
            .map_err(Error::Http)?;
        let resp = check_response(resp).await?;
        Ok(Box::pin(normalize_responses_stream(sse_stream(
            resp.bytes_stream(),
        ))))
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    async fn chat_stream(&self, req: ChatRequest) -> Result<EventStream> {
        match self.wire {
            WireApi::Chat => self.chat_completions(req).await,
            WireApi::Responses => self.responses(req).await,
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn wire_api(&self) -> &'static str {
        self.wire.as_str()
    }
}

/// Message → chat 协议消息对象。
fn chat_message(m: &Message) -> Value {
    match m.role {
        Role::System => json!({"role": "system", "content": m.content}),
        Role::User => json!({"role": "user", "content": m.content}),
        Role::Assistant => match &m.tool_calls {
            Some(tcs) => {
                let calls: Vec<Value> = tcs
                    .iter()
                    .map(|t| {
                        json!({
                            "id": t.id,
                            "type": "function",
                            "function": {"name": t.name, "arguments": t.arguments}
                        })
                    })
                    .collect();
                json!({"role": "assistant", "content": m.content, "tool_calls": calls})
            }
            None => json!({"role": "assistant", "content": m.content}),
        },
        Role::Tool => json!({
            "role": "tool",
            "tool_call_id": m.tool_call_id.clone().unwrap_or_default(),
            "content": m.content
        }),
    }
}

async fn check_response(resp: reqwest::Response) -> Result<reqwest::Response> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    let body = resp.text().await.unwrap_or_default();
    let snippet: String = body.chars().take(600).collect();
    Err(Error::Provider(format!(
        "HTTP {} from provider: {snippet}",
        status.as_u16()
    )))
}

/// chat 协议 SSE → 归一化事件。
///
/// delta 中 `content` / `reasoning_content` 直接转 Token / Reasoning；
/// `tool_calls` 按 `index` 累积 id/name/arguments，流结束时整条发出；
/// 末尾 `usage` 与 `[DONE]` 分别转 Usage / Done。
pub(crate) fn normalize_chat_stream<S>(
    sse: S,
) -> impl Stream<Item = Result<StreamEvent>> + Send
where
    S: Stream<Item = Result<SseEvent>> + Send + 'static,
{
    async_stream::stream! {
        futures_util::pin_mut!(sse);
        let mut tool_calls: BTreeMap<u64, (String, String, String)> = BTreeMap::new();
        let mut usage: Option<(u32, u32)> = None;

        while let Some(ev) = sse.next().await {
            let ev = match ev {
                Ok(e) => e,
                Err(e) => {
                    yield Err(e);
                    return;
                }
            };
            let data = ev.data.trim();
            if data.is_empty() {
                continue;
            }
            if data == "[DONE]" {
                break;
            }
            let v: Value = match serde_json::from_str(data) {
                Ok(v) => v,
                Err(_) => continue,
            };

            if let Some(u) = v.get("usage").filter(|u| !u.is_null()) {
                let input = as_u32(&u["prompt_tokens"]).or_else(|| as_u32(&u["input_tokens"]));
                let output = as_u32(&u["completion_tokens"]).or_else(|| as_u32(&u["output_tokens"]));
                if let (Some(i), Some(o)) = (input, output) {
                    usage = Some((i, o));
                }
            }

            let Some(choice) = v["choices"].get(0).filter(|c| !c.is_null()) else {
                continue;
            };
            let delta = &choice["delta"];

            if let Some(c) = delta["content"].as_str().filter(|s| !s.is_empty()) {
                yield Ok(StreamEvent::Token(c.to_string()));
            }
            let reasoning = delta["reasoning_content"]
                .as_str()
                .filter(|s| !s.is_empty())
                .or_else(|| delta["reasoning"].as_str().filter(|s| !s.is_empty()));
            if let Some(r) = reasoning {
                yield Ok(StreamEvent::Reasoning(r.to_string()));
            }
            if let Some(arr) = delta["tool_calls"].as_array() {
                for tc in arr {
                    let idx = tc["index"].as_u64().unwrap_or(0);
                    let entry = tool_calls
                        .entry(idx)
                        .or_insert_with(|| (String::new(), String::new(), String::new()));
                    if let Some(id) = tc["id"].as_str().filter(|s| !s.is_empty()) {
                        entry.0 = id.to_string();
                    }
                    if let Some(n) = tc["function"]["name"].as_str().filter(|s| !s.is_empty()) {
                        entry.1 = n.to_string();
                    }
                    if let Some(a) = tc["function"]["arguments"].as_str() {
                        entry.2.push_str(a);
                    }
                }
            }
        }

        for (_, (id, name, arguments)) in tool_calls {
            let id = if id.is_empty() {
                format!("call_{}", Uuid::new_v4().simple())
            } else {
                id
            };
            yield Ok(StreamEvent::ToolCall { id, name, arguments });
        }
        if let Some((input_tokens, output_tokens)) = usage {
            yield Ok(StreamEvent::Usage {
                input_tokens,
                output_tokens,
            });
        }
        yield Ok(StreamEvent::Done);
    }
}

/// responses 协议 SSE → 归一化事件。
///
/// 类型化事件：`response.output_text.delta` → Token；
/// `response.reasoning_*_text.delta` → Reasoning；
/// `response.output_item.done`（function_call 项，arguments 完整）→ ToolCall；
/// `response.completed` → Usage + Done；`response.failed` / `error` → 终止性错误。
pub(crate) fn normalize_responses_stream<S>(
    sse: S,
) -> impl Stream<Item = Result<StreamEvent>> + Send
where
    S: Stream<Item = Result<SseEvent>> + Send + 'static,
{
    async_stream::stream! {
        futures_util::pin_mut!(sse);
        let mut tool_calls: Vec<(String, String, String)> = Vec::new();
        let mut usage: Option<(u32, u32)> = None;

        while let Some(ev) = sse.next().await {
            let ev = match ev {
                Ok(e) => e,
                Err(e) => {
                    yield Err(e);
                    return;
                }
            };
            let data = ev.data.trim();
            if data.is_empty() {
                continue;
            }
            let v: Value = match serde_json::from_str(data) {
                Ok(v) => v,
                Err(_) => continue,
            };
            // 优先取 SSE event 名；部分兼容端点省略 event 行，类型在 data.type 中
            let etype = ev
                .event
                .clone()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| v["type"].as_str().unwrap_or("").to_string());

            match etype.as_str() {
                "response.output_text.delta" => {
                    if let Some(d) = v["delta"].as_str().filter(|s| !s.is_empty()) {
                        yield Ok(StreamEvent::Token(d.to_string()));
                    }
                }
                "response.reasoning_summary_text.delta" | "response.reasoning_text.delta" => {
                    if let Some(d) = v["delta"].as_str().filter(|s| !s.is_empty()) {
                        yield Ok(StreamEvent::Reasoning(d.to_string()));
                    }
                }
                "response.output_item.done" => {
                    let item = &v["item"];
                    if item["type"].as_str() == Some("function_call") {
                        tool_calls.push((
                            item["call_id"]
                                .as_str()
                                .or_else(|| item["id"].as_str())
                                .unwrap_or("")
                                .to_string(),
                            item["name"].as_str().unwrap_or("").to_string(),
                            item["arguments"].as_str().unwrap_or("").to_string(),
                        ));
                    }
                }
                "response.completed" => {
                    let u = &v["response"]["usage"];
                    if !u.is_null() {
                        usage = Some((
                            as_u32(&u["input_tokens"]).unwrap_or(0),
                            as_u32(&u["output_tokens"]).unwrap_or(0),
                        ));
                    }
                }
                "response.failed" => {
                    let msg = v["response"]["error"]["message"]
                        .as_str()
                        .unwrap_or("response failed")
                        .to_string();
                    yield Err(Error::Provider(msg));
                    return;
                }
                "response.incomplete" => {
                    let reason = v["response"]["incomplete_details"]["reason"]
                        .as_str()
                        .unwrap_or("incomplete");
                    yield Err(Error::Provider(format!("response incomplete: {reason}")));
                    return;
                }
                "error" => {
                    let msg = v["message"]
                        .as_str()
                        .or_else(|| v["error"]["message"].as_str())
                        .unwrap_or("stream error")
                        .to_string();
                    yield Err(Error::Provider(msg));
                    return;
                }
                _ => {}
            }
        }

        for (id, name, arguments) in tool_calls {
            let id = if id.is_empty() {
                format!("call_{}", Uuid::new_v4().simple())
            } else {
                id
            };
            yield Ok(StreamEvent::ToolCall { id, name, arguments });
        }
        if let Some((input_tokens, output_tokens)) = usage {
            yield Ok(StreamEvent::Usage {
                input_tokens,
                output_tokens,
            });
        }
        yield Ok(StreamEvent::Done);
    }
}

fn as_u32(v: &Value) -> Option<u32> {
    v.as_u64().map(|n| n.min(u32::MAX as u64) as u32)
}

/// 供测试使用：构造 ToolSpec（避免测试代码重复）。
#[allow(dead_code)]
pub(crate) fn tool_spec(name: &str, description: &str) -> ToolSpec {
    ToolSpec {
        name: name.to_string(),
        description: description.to_string(),
        parameters: json!({"type": "object", "properties": {}}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::stream;

    fn ev(event: Option<&str>, data: &str) -> Result<SseEvent> {
        Ok(SseEvent {
            event: event.map(Into::into),
            data: data.into(),
        })
    }

    async fn collect_events<S>(s: S) -> Vec<StreamEvent>
    where
        S: Stream<Item = Result<StreamEvent>> + Send + 'static,
    {
        futures_util::pin_mut!(s);
        let mut out = Vec::new();
        while let Some(e) = s.next().await {
            out.push(e.unwrap());
        }
        out
    }

    #[tokio::test]
    async fn chat_stream_normalizes_tokens_tool_calls_usage() {
        let sse = stream::iter(vec![
            ev(None, r#"{"choices":[{"delta":{"content":"Hel"}}]}"#),
            ev(None, r#"{"choices":[{"delta":{"reasoning_content":"hmm"}}]}"#),
            ev(None, r#"{"choices":[{"delta":{"content":"lo"}}]}"#),
            ev(
                None,
                r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","function":{"name":"read_file","arguments":"{\"path\":"}}]}}]}"#,
            ),
            ev(
                None,
                r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"a.rs\"}"}}]}}]}"#,
            ),
            ev(None, r#"{"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#),
            ev(None, r#"{"choices":[],"usage":{"prompt_tokens":10,"completion_tokens":5}}"#),
            ev(None, "[DONE]"),
        ]);
        let out = collect_events(normalize_chat_stream(sse)).await;

        assert!(matches!(&out[0], StreamEvent::Token(t) if t == "Hel"));
        assert!(matches!(&out[1], StreamEvent::Reasoning(r) if r == "hmm"));
        assert!(matches!(&out[2], StreamEvent::Token(t) if t == "lo"));
        let (name, args) = out
            .iter()
            .find_map(|e| match e {
                StreamEvent::ToolCall { name, arguments, .. } => {
                    Some((name.clone(), arguments.clone()))
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(name, "read_file");
        assert_eq!(args, r#"{"path":"a.rs"}"#);
        assert!(out.iter().any(|e| matches!(
            e,
            StreamEvent::Usage {
                input_tokens: 10,
                output_tokens: 5
            }
        )));
        assert!(matches!(out.last().unwrap(), StreamEvent::Done));
    }

    #[tokio::test]
    async fn responses_stream_normalizes() {
        let sse = stream::iter(vec![
            ev(
                Some("response.output_text.delta"),
                r#"{"type":"response.output_text.delta","delta":"Hi"}"#,
            ),
            ev(
                Some("response.reasoning_summary_text.delta"),
                r#"{"type":"response.reasoning_summary_text.delta","delta":"think"}"#,
            ),
            ev(
                Some("response.output_item.done"),
                r#"{"type":"response.output_item.done","item":{"type":"function_call","call_id":"call_x","name":"shell","arguments":"{\"command\":\"ls\"}"}}"#,
            ),
            ev(
                Some("response.completed"),
                r#"{"type":"response.completed","response":{"usage":{"input_tokens":7,"output_tokens":3}}}"#,
            ),
        ]);
        let out = collect_events(normalize_responses_stream(sse)).await;

        assert!(matches!(&out[0], StreamEvent::Token(t) if t == "Hi"));
        assert!(matches!(&out[1], StreamEvent::Reasoning(r) if r == "think"));
        let (name, args) = out
            .iter()
            .find_map(|e| match e {
                StreamEvent::ToolCall { name, arguments, .. } => {
                    Some((name.clone(), arguments.clone()))
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(name, "shell");
        assert_eq!(args, r#"{"command":"ls"}"#);
        assert!(out.iter().any(|e| matches!(
            e,
            StreamEvent::Usage {
                input_tokens: 7,
                output_tokens: 3
            }
        )));
        assert!(matches!(out.last().unwrap(), StreamEvent::Done));
    }

    #[tokio::test]
    async fn responses_failure_event_yields_error() {
        let sse = stream::iter(vec![ev(
            Some("response.failed"),
            r#"{"type":"response.failed","response":{"error":{"message":"boom"}}}"#,
        )]);
        let normalized = normalize_responses_stream(sse);
        futures_util::pin_mut!(normalized);
        match normalized.next().await {
            Some(Err(Error::Provider(msg))) => assert!(msg.contains("boom")),
            other => panic!("expected provider error, got {other:?}"),
        }
    }

    #[test]
    fn chat_message_shapes() {
        let user = Message::user("hello");
        assert_eq!(chat_message(&user)["role"], "user");

        let call = crate::memory::ToolCall {
            id: "call_1".into(),
            name: "shell".into(),
            arguments: "{}".into(),
        };
        let assistant = Message::assistant("", Some(vec![call]));
        let v = chat_message(&assistant);
        assert_eq!(v["tool_calls"][0]["function"]["name"], "shell");

        let tool = Message::tool_result("call_1", "shell", "ok");
        assert_eq!(chat_message(&tool)["tool_call_id"], "call_1");
    }
}
