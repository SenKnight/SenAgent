//! 工具系统：注册表 + JSON Schema 生成 + 内置工具。
//!
//! 每个工具实现 [`Tool`]，参数结构体用 serde 反序列化、用 schemars 生成
//! JSON Schema 提交给 LLM；执行错误会以文本形式回传给模型（让它自我纠正）。

pub mod builtin;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::error::{Error, Result};
use crate::providers::ToolSpec;

/// 一个可被 LLM 调用的工具。
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    /// 参数 JSON Schema（由 schemars 从入参结构体生成）
    fn parameters_schema(&self) -> Value;
    /// 执行工具；`base` 为当前工作目录（工具内的相对路径以此为根）。
    /// Ok 返回观察结果文本，Err 的消息会作为错误反馈给模型。
    async fn execute(&self, args: Value, base: &Path) -> Result<String>;
}

/// 将可能为相对路径的 `path` 解析到工作目录 `base` 下（绝对路径原样返回）。
pub(crate) fn resolve_path(base: &Path, path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        base.join(p)
    }
}

/// 工具注册表。
#[derive(Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        self.tools.insert(tool.name().to_string(), Arc::new(tool));
    }

    pub fn register_arc(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    pub fn names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// 转换为提交给 LLM 的工具定义。
    pub fn specs(&self) -> Vec<ToolSpec> {
        self.tools
            .values()
            .map(|t| ToolSpec {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters_schema(),
            })
            .collect()
    }

    /// 仅返回白名单内工具的定义（顺序按名称）。
    pub fn specs_allowed(&self, allowed: &[&str]) -> Vec<ToolSpec> {
        self.tools
            .values()
            .filter(|t| allowed.contains(&t.name()))
            .map(|t| ToolSpec {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters_schema(),
            })
            .collect()
    }

    /// 执行工具调用（name + JSON 字符串参数 + 当前工作目录）。
    pub async fn execute(&self, name: &str, arguments_json: &str, base: &Path) -> Result<String> {
        let tool = self
            .get(name)
            .ok_or_else(|| Error::Tool(format!("unknown tool: {name}")))?;
        let args: Value = if arguments_json.trim().is_empty() {
            Value::Object(Default::default())
        } else {
            serde_json::from_str(arguments_json)
                .map_err(|e| Error::Tool(format!("invalid tool arguments JSON: {e}")))?
        };
        tool.execute(args, base).await
    }
}

/// 由 schemars 生成 JSON Schema 并清理元字段。
pub(crate) fn schema_of<T: schemars::JsonSchema>() -> Value {
    let mut v = serde_json::to_value(schemars::schema_for!(T))
        .unwrap_or_else(|_| serde_json::json!({"type": "object"}));
    if let Some(obj) = v.as_object_mut() {
        obj.remove("$schema");
        obj.remove("title");
    }
    v
}
