//! LLM Provider 层：协议适配器模式。
//!
//! `WireApi::Chat`（/v1/chat/completions）与 `WireApi::Responses`（/v1/responses）
//! 共享统一的 [`StreamEvent`] 归一化事件模型，内核只消费归一化事件；
//! 后续扩展 Anthropic Messages 等协议只需新增适配器，不触碰内核。

mod openai;
mod sse;

use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures_util::Stream;
use serde::{Deserialize, Serialize};

use crate::config::ProviderConfig;
use crate::error::Result;
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
