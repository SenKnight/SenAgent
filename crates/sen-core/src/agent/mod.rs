//! Agent 主循环：流式响应 + 多轮工具调用 + 上下文截断。
//!
//! [`Agent::run_turn`] 输入一条用户消息，输出 [`AgentEvent`] 事件流：
//! 文本/推理增量、工具调用与结果、结束与错误。工具调用在内部循环执行，
//! 消息实时持久化到 SQLite，可随时中断（丢弃流）后恢复。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_stream::stream;
use futures_util::StreamExt;

use crate::config::Config;
use crate::error::Result;
use crate::events::{AgentEvent, Usage};
use crate::memory::{Message, Store, ToolCall};
use crate::paths;
use crate::prompts;
use crate::providers::{self, ChatRequest, Provider, StreamEvent};
use crate::skills::SkillManager;
use crate::tools::{builtin, ToolRegistry};
use crate::util::{estimate_tokens, trim_messages};

/// Agent：一次构造，多次 `run_turn`。
#[derive(Clone)]
pub struct Agent {
    provider: Arc<dyn Provider>,
    tools: Arc<ToolRegistry>,
    store: Arc<Store>,
    skills: Arc<SkillManager>,
    config: Arc<Config>,
    cwd: PathBuf,
}

impl Agent {
    /// 灵活构造（嵌入 / 测试场景由调用方提供全部部件）。
    pub fn new(
        provider: Arc<dyn Provider>,
        tools: ToolRegistry,
        store: Store,
        skills: Arc<SkillManager>,
        config: Config,
        cwd: PathBuf,
    ) -> Self {
        Self {
            provider,
            tools: Arc::new(tools),
            store: Arc::new(store),
            skills,
            config: Arc::new(config),
            cwd,
        }
    }

    /// 生产构造：打开 `~/.sen-agent/data.db`、扫描 `~/.agents/skills`、
    /// 注册全部内置工具并按配置创建 provider。
    pub fn from_config(config: Config, provider_name: Option<&str>, cwd: PathBuf) -> Result<Self> {
        if let Err(e) = paths::ensure_base_dir() {
            tracing::warn!("无法创建数据目录: {e}");
        }
        if let Err(e) = std::fs::create_dir_all(paths::skills_dir()) {
            tracing::warn!("无法创建技能目录: {e}");
        }
        let provider_cfg = config.resolve_provider(provider_name)?;
        let provider = providers::create_provider(&provider_cfg)?;
        let store = Store::open(&paths::db_path())?;
        let skills = Arc::new(SkillManager::new(paths::skills_dir()));
        let mut registry = ToolRegistry::new();
        builtin::register_builtin_tools(&mut registry, skills.clone());
        Ok(Self::new(provider, registry, store, skills, config, cwd))
    }

    /// 组装当前系统提示词（每轮刷新：指导文件与技能索引实时生效）。
    pub fn system_prompt(&self) -> String {
        let guidance = prompts::load_guidance(&self.cwd);
        let skills_index = self.skills.index_prompt();
        prompts::assemble_system_prompt(
            self.config.system_prompt.as_deref(),
            &guidance,
            &skills_index,
        )
    }

    pub fn provider(&self) -> &Arc<dyn Provider> {
        &self.provider
    }

    pub fn tools(&self) -> &Arc<ToolRegistry> {
        &self.tools
    }

    pub fn store(&self) -> &Arc<Store> {
        &self.store
    }

    pub fn skills(&self) -> &Arc<SkillManager> {
        &self.skills
    }

    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    /// 执行一轮用户输入：返回事件流，内部完成多轮工具调用循环并持久化。
    ///
    /// 取消方式：丢弃（drop）返回的流即可中断，已产生的消息均已落库。
    pub fn run_turn(
        &self,
        session_id: &str,
        input: &str,
    ) -> impl futures_util::Stream<Item = AgentEvent> + Send + 'static {
        let agent = self.clone();
        let session_id = session_id.to_string();
        let input = input.to_string();

        stream! {
            // 1. 持久化用户消息
            let user_msg = Message::user(&input);
            if let Err(e) = agent.store.append_message(&session_id, &user_msg) {
                yield AgentEvent::Error { message: format!("保存用户消息失败: {e}") };
                return;
            }
            if let Err(e) = agent.store.maybe_set_title(&session_id, &input) {
                tracing::warn!("设置会话标题失败: {e}");
            }

            // 2. 系统提示词 + 历史消息
            let system = agent.system_prompt();
            let mut messages = match agent.store.load_messages(&session_id) {
                Ok(m) => m,
                Err(e) => {
                    yield AgentEvent::Error { message: format!("加载历史消息失败: {e}") };
                    return;
                }
            };

            let tool_specs = agent.tools.specs();
            let max_rounds = agent.config.max_tool_rounds.max(1);
            let mut total_usage = Usage::default();

            // 3. 工具调用循环
            for _round in 0..max_rounds {
                let input_budget = agent
                    .config
                    .context_budget()
                    .saturating_sub(estimate_tokens(&system))
                    .max(1_024);
                let history = trim_messages(&messages, input_budget);

                let req = ChatRequest {
                    model: agent.provider.model().to_string(),
                    system: Some(system.clone()),
                    messages: history,
                    tools: tool_specs.clone(),
                    max_tokens: None,
                    temperature: None,
                };
                let mut stream = match agent.provider.chat_stream(req).await {
                    Ok(s) => s,
                    Err(e) => {
                        yield AgentEvent::Error { message: format!("请求模型失败: {e}") };
                        return;
                    }
                };

                let mut text = String::new();
                let mut calls: Vec<ToolCall> = Vec::new();
                let mut round_usage = Usage::default();
                let mut stream_error: Option<String> = None;

                while let Some(ev) = stream.next().await {
                    match ev {
                        Ok(StreamEvent::Token(t)) => {
                            text.push_str(&t);
                            yield AgentEvent::Token { delta: t };
                        }
                        Ok(StreamEvent::Reasoning(r)) => {
                            yield AgentEvent::Reasoning { delta: r };
                        }
                        Ok(StreamEvent::ToolCall { id, name, arguments }) => {
                            yield AgentEvent::ToolCall {
                                name: name.clone(),
                                arguments: arguments.clone(),
                            };
                            calls.push(ToolCall { id, name, arguments });
                        }
                        Ok(StreamEvent::Usage { input_tokens, output_tokens }) => {
                            round_usage = Usage { input_tokens, output_tokens };
                        }
                        Ok(StreamEvent::Done) => break,
                        Err(e) => {
                            stream_error = Some(e.to_string());
                            break;
                        }
                    }
                }

                total_usage.input_tokens += round_usage.input_tokens;
                total_usage.output_tokens += round_usage.output_tokens;

                if let Some(e) = stream_error {
                    // 已产生的部分文本仍然落库，重启后可恢复上下文
                    if !text.is_empty() {
                        let _ = agent
                            .store
                            .append_message(&session_id, &Message::assistant(&text, None));
                    }
                    yield AgentEvent::Error { message: format!("模型流中断: {e}") };
                    return;
                }

                // 持久化 assistant 消息（正文 + 工具调用）
                let assistant = Message::assistant(
                    &text,
                    if calls.is_empty() { None } else { Some(calls.clone()) },
                );
                if let Err(e) = agent.store.append_message(&session_id, &assistant) {
                    yield AgentEvent::Error { message: format!("保存助手消息失败: {e}") };
                    return;
                }
                messages.push(assistant);

                // 无工具调用：本轮结束
                if calls.is_empty() {
                    yield AgentEvent::Done {
                        usage: if total_usage.total() > 0 { Some(total_usage) } else { None },
                    };
                    return;
                }

                // 执行工具并持久化结果（串行保序）
                for call in &calls {
                    let (output, is_error) =
                        match agent.tools.execute(&call.name, &call.arguments).await {
                            Ok(out) => (out, false),
                            Err(e) => (format!("Error: {e}"), true),
                        };
                    yield AgentEvent::ToolResult {
                        name: call.name.clone(),
                        output: output.clone(),
                        is_error,
                    };
                    let tool_msg = Message::tool_result(&call.id, &call.name, &output);
                    if let Err(e) = agent.store.append_message(&session_id, &tool_msg) {
                        yield AgentEvent::Error { message: format!("保存工具结果失败: {e}") };
                        return;
                    }
                    messages.push(tool_msg);
                }
            }

            yield AgentEvent::Error {
                message: format!(
                    "已达到最大工具调用轮数（{max_rounds}），本轮终止；可继续输入让 Agent 接着处理"
                ),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use super::*;
    use crate::memory::Role;
    use crate::providers::EventStream;
    use crate::tools::Tool;
    use serde_json::Value;

    /// 脚本化 Mock Provider：每次 chat_stream 弹出一个小剧本。
    struct MockProvider {
        scripts: Mutex<VecDeque<Vec<StreamEvent>>>,
    }

    #[async_trait::async_trait]
    impl Provider for MockProvider {
        async fn chat_stream(&self, _req: ChatRequest) -> Result<EventStream> {
            let events = self
                .scripts
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_default();
            Ok(Box::pin(futures_util::stream::iter(
                events.into_iter().map(Ok::<_, crate::error::Error>),
            )))
        }

        fn name(&self) -> &str {
            "mock"
        }

        fn model(&self) -> &str {
            "mock-model"
        }

        fn wire_api(&self) -> &'static str {
            "chat"
        }
    }

    struct EchoTool;

    #[async_trait::async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &'static str {
            "echo"
        }

        fn description(&self) -> &'static str {
            "回显 text 参数"
        }

        fn parameters_schema(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": { "text": { "type": "string" } }
            })
        }

        async fn execute(&self, args: Value) -> Result<String> {
            Ok(format!(
                "echo: {}",
                args.get("text").and_then(|v| v.as_str()).unwrap_or("")
            ))
        }
    }

    #[tokio::test]
    async fn tool_loop_and_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("t.db")).unwrap();
        let session = store.create_session("").unwrap();

        let provider = Arc::new(MockProvider {
            scripts: Mutex::new(VecDeque::from(vec![
                // 第 1 轮：文本 + 工具调用
                vec![
                    StreamEvent::Token("让我调用工具。".into()),
                    StreamEvent::ToolCall {
                        id: "c1".into(),
                        name: "echo".into(),
                        arguments: r#"{ "text": "hi" }"#.into(),
                    },
                    StreamEvent::Done,
                ],
                // 第 2 轮：最终回答 + 用量
                vec![
                    StreamEvent::Token("结果是 hi。".into()),
                    StreamEvent::Usage {
                        input_tokens: 10,
                        output_tokens: 5,
                    },
                    StreamEvent::Done,
                ],
            ])),
        });

        let mut registry = ToolRegistry::new();
        registry.register(EchoTool);
        let skills = Arc::new(SkillManager::new(dir.path().join("skills")));

        let agent = Agent::new(
            provider,
            registry,
            store,
            skills,
            Config::default(),
            dir.path().to_path_buf(),
        );

        let events: Vec<AgentEvent> = agent.run_turn(&session.id, "帮我 echo 一下").collect().await;

        assert!(events
            .iter()
            .any(|e| matches!(e, AgentEvent::ToolCall { name, .. } if name == "echo")));
        assert!(events.iter().any(|e| matches!(
            e,
            AgentEvent::ToolResult { output, is_error: false, .. } if output == "echo: hi"
        )));
        match events.last().unwrap() {
            AgentEvent::Done { usage: Some(u) } => {
                assert_eq!(u.input_tokens, 10);
                assert_eq!(u.output_tokens, 5);
            }
            other => panic!("期望 Done 事件，实际: {other:?}"),
        }

        // 持久化校验：user → assistant(tool_calls) → tool → assistant
        let msgs = agent.store.load_messages(&session.id).unwrap();
        assert_eq!(msgs.len(), 4);
        assert_eq!(msgs[0].role, Role::User);
        assert!(msgs[1].tool_calls.is_some());
        assert_eq!(msgs[2].role, Role::Tool);
        assert_eq!(msgs[2].content, "echo: hi");
        assert_eq!(msgs[3].content, "结果是 hi。");

        let s = agent.store.get_session(&session.id).unwrap().unwrap();
        assert_eq!(s.title, "帮我 echo 一下");
    }

    #[tokio::test]
    async fn plain_reply_without_tools() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("t.db")).unwrap();
        let session = store.create_session("").unwrap();

        let provider = Arc::new(MockProvider {
            scripts: Mutex::new(VecDeque::from(vec![vec![
                StreamEvent::Token("你好！".into()),
                StreamEvent::Done,
            ]])),
        });

        let agent = Agent::new(
            provider,
            ToolRegistry::new(),
            store,
            Arc::new(SkillManager::new(dir.path().join("skills"))),
            Config::default(),
            dir.path().to_path_buf(),
        );

        let events: Vec<AgentEvent> = agent.run_turn(&session.id, "你好").collect().await;
        assert!(matches!(
            events.last().unwrap(),
            AgentEvent::Done { .. }
        ));
        let msgs = agent.store.load_messages(&session.id).unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[1].content, "你好！");
    }

    #[test]
    fn system_prompt_refreshes_skills_index() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("t.db")).unwrap();
        let skills = Arc::new(SkillManager::new(dir.path().join("skills")));
        skills
            .create("git-helper", None, Some("处理 git 操作"), None)
            .unwrap();

        let provider = Arc::new(MockProvider {
            scripts: Mutex::new(VecDeque::new()),
        });
        let agent = Agent::new(
            provider,
            ToolRegistry::new(),
            store,
            skills,
            Config::default(),
            dir.path().to_path_buf(),
        );

        let sp = agent.system_prompt();
        assert!(sp.contains("SenAgent"));
        assert!(sp.contains("git-helper"));
        assert!(sp.contains("处理 git 操作"));
    }
}
