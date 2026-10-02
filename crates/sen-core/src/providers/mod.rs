//! LLM Provider 层：协议适配器模式。
//!
//! `WireApi::Chat`（/v1/chat/completions）与 `WireApi::Responses`（/v1/responses）
//! 共享统一的 [`StreamEvent`] 归一化事件模型，内核只消费归一化事件；
//! 后续扩展 Anthropic Messages 等协议只需新增适配器，不触碰内核。

mod openai;
mod sse;

use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::Stream;
use serde::{Deserialize, Serialize};

use crate::config::ProviderConfig;
use crate::error::{Error, Result};
use crate::memory::Message;

pub use openai::OpenAiProvider;
pub use sse::{sse_stream, SseEvent};

/// Provider 层的原始流式事件（归一化模型）。
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// 正文文本增量
    Token(String),
    /// 推理过程增量
    Reasoning(String),
    /// 完整的工具调用（流式分片已在适配器内部累积完毕）
    ToolCall {
        id: String,
        name: String,
        arguments: String,
    },
    /// Token 用量
    Usage {
        input_tokens: u32,
        output_tokens: u32,
    },
    /// 本次生成正常结束
    Done,
}

/// 工具定义（提交给 LLM 的 JSON Schema）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    /// 由 schemars 从参数结构体自动生成
    pub parameters: serde_json::Value,
}

/// 一次模型调用请求。
#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub model: String,
    /// 系统提示词（chat 协议 → system 消息；responses 协议 → instructions）
    pub system: Option<String>,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolSpec>,
    /// 单次请求覆盖的最大输出 token（默认取 provider 配置）
    pub max_tokens: Option<u32>,
    /// 单次请求覆盖的温度（默认取 provider 配置）
    pub temperature: Option<f32>,
}

pub type EventStream = Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>;

/// LLM Provider 抽象。
#[async_trait]
pub trait Provider: Send + Sync {
    /// 发起流式对话，返回归一化事件流。
    async fn chat_stream(&self, req: ChatRequest) -> Result<EventStream>;
    /// provider 名称（配置中的 name）
    fn name(&self) -> &str;
    /// 当前默认模型
    fn model(&self) -> &str;
    /// 当前线协议（chat / responses）
    fn wire_api(&self) -> &'static str;
}

/// 根据配置构建 provider 实例。
pub fn create_provider(cfg: &ProviderConfig) -> Result<Arc<dyn Provider>> {
    Ok(Arc::new(OpenAiProvider::new(cfg)?))
}

/// 自动发现 provider 可用模型列表（`GET {base_url}/models`）。
///
/// 兼容 OpenAI 兼容端点的 `{ "data": [{ "id": ... }] }`，
/// 以及 Ollama 等返回的 `{ "models": [{ "name" | "id" | "model": ... }] }`，
/// 也接受纯数组 `["a", "b"]`。仅只读查询，不产生副作用。
pub async fn list_models(cfg: &ProviderConfig) -> Result<Vec<String>> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .build()
        .map_err(Error::Http)?;
    let base = cfg.base_url.trim_end_matches('/');
    let mut rb = client.get(format!("{base}/models"));
    if let Some(key) = cfg.resolved_api_key() {
        rb = rb.bearer_auth(key);
    }
    for (k, v) in &cfg.headers {
        rb = rb.header(k.as_str(), v.as_str());
    }
    let resp = rb.send().await.map_err(Error::Http)?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        let snippet: String = text.chars().take(300).collect();
        return Err(Error::Provider(format!(
            "HTTP {} from {base}/models: {snippet}",
            status.as_u16()
        )));
    }
    let v: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| Error::Provider(format!("模型列表解析失败: {e}")))?;

    let mut out: Vec<String> = Vec::new();
    // OpenAI 兼容：{ "data": [ { "id": "..." } ] }
    if let Some(arr) = v.get("data").and_then(|d| d.as_array()) {
        for item in arr {
            if let Some(id) = item.get("id").and_then(|x| x.as_str()) {
                out.push(id.to_string());
            }
        }
    }
    // Ollama 等：{ "models": [ { "name" | "id" | "model": "..." } ] }
    if out.is_empty() {
        if let Some(arr) = v.get("models").and_then(|d| d.as_array()) {
            for item in arr {
                let id = item
                    .get("id")
                    .and_then(|x| x.as_str())
                    .or_else(|| item.get("name").and_then(|x| x.as_str()))
                    .or_else(|| item.get("model").and_then(|x| x.as_str()));
                if let Some(id) = id {
                    out.push(id.to_string());
                }
            }
        }
    }
    // 纯数组：["a", "b"] 或 [{ "id": "a" }]
    if out.is_empty() {
        if let Some(arr) = v.as_array() {
            for item in arr {
                if let Some(s) = item.as_str() {
                    out.push(s.to_string());
                } else if let Some(id) = item.get("id").and_then(|x| x.as_str()) {
                    out.push(id.to_string());
                }
            }
        }
    }

    out.sort();
    out.dedup();
    Ok(out)
}
