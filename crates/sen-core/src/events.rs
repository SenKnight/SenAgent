//! 面向消费者的 Agent 事件模型。
//!
//! Provider 层的原始流式事件（[`crate::providers::StreamEvent`]）经 Agent 主循环
//! 归一化为 [`AgentEvent`]，由 CLI / WebSocket / 桌面端统一消费。
//! 序列化格式（`type` 字段区分）即 WebSocket 协议格式。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

impl Usage {
    pub fn total(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

/// Agent 单轮对话产生的事件流。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// 正文文本增量
    Token { delta: String },
    /// 推理过程增量（responses 协议推理项 / chat 协议 reasoning_content）
    Reasoning { delta: String },
    /// 即将执行的工具调用
    ToolCall { name: String, arguments: String },
    /// 工具执行结果
    ToolResult {
        name: String,
        output: String,
        is_error: bool,
    },
    /// 计划模式下产出的计划（非终止事件，`Done` 前发出）
    Plan { content: String },
    /// 本轮结束
    Done { usage: Option<Usage> },
    /// 发生错误，本轮终止
    Error { message: String },
}

impl AgentEvent {
    pub fn is_terminal(&self) -> bool {
        matches!(self, AgentEvent::Done { .. } | AgentEvent::Error { .. })
    }
}
