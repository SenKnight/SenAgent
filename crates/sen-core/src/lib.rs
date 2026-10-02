//! SenAgent core library.
//!
//! 组织方式：
//! - [`agent`]：Agent 主循环（流式响应 + 多轮工具调用 + 上下文截断）
//! - [`providers`]：LLM Provider 层（wire_api = chat / responses 双协议适配器）
//! - [`tools`]：工具注册表与内置工具（shell、文件、搜索、web、skill）
//! - [`skills`]：`~/.agents/skills` 分层技能系统
//! - [`prompts`]：系统提示词组装（AGENTS.md 全局/项目级指导）
//! - [`memory`]：SQLite 会话与消息持久化
//! - [`config`] / [`paths`]：配置加载与目录约定

pub mod agent;
pub mod config;
pub mod error;
pub mod events;
pub mod memory;
pub mod paths;
pub mod prompts;
pub mod providers;
pub mod skills;
pub mod tools;
pub mod util;

pub use agent::{Agent, TurnMode};
pub use config::{Config, ProviderConfig, WireApi};
pub use error::{Error, Result};
pub use events::{AgentEvent, Usage};
pub use memory::{Message, Plan, Role, Session, Store, ToolCall};
pub use providers::{ChatRequest, Provider, StreamEvent, ToolSpec};
pub use skills::{Skill, SkillManager};
