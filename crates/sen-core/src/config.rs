//! 配置加载与管理。
//!
//! 配置文件：`~/.sen-agent/config.toml`（首次运行自动生成模板）。
//! 环境变量覆盖：`SEN_PROVIDER` / `SEN_BASE_URL` / `SEN_API_KEY` / `SEN_MODEL` / `SEN_WIRE_API`。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::paths;

/// 线协议类型：决定与 LLM 端点交互时使用的 API 形态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WireApi {
    /// OpenAI Chat Completions（/v1/chat/completions）：兼容端点最广，
    /// 覆盖 OpenAI / DeepSeek / Kimi / Qwen / GLM / Ollama 等。
    #[default]
    Chat,
    /// OpenAI Responses（/v1/responses）：OpenAI 原生协议，
    /// 支持状态化会话、推理项（reasoning items）与内置工具。
    Responses,
}

impl WireApi {
    pub fn as_str(&self) -> &'static str {
        match self {
            WireApi::Chat => "chat",
            WireApi::Responses => "responses",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "chat" | "chat_completions" | "chat-completions" => Ok(WireApi::Chat),
            "responses" => Ok(WireApi::Responses),
            other => Err(Error::Config(format!(
                "unknown wire_api: {other} (expected \"chat\" or \"responses\")"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// 提供商标识（用户自定义，例如 openai / deepseek / ollama）
    pub name: String,
    /// API 根地址，例如 https://api.openai.com/v1
    pub base_url: String,
    /// API Key；支持 `env:VAR_NAME` 形式从环境变量读取
    #[serde(default)]
    pub api_key: Option<String>,
    /// 默认模型
    pub model: String,
    /// 线协议：chat（默认）或 responses
    #[serde(default)]
    pub wire_api: WireApi,
    /// 附加请求头
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// 最大输出 token（可选）
    #[serde(default)]
    pub max_tokens: Option<u32>,
    /// 采样温度（可选）
    #[serde(default)]
    pub temperature: Option<f32>,
}

impl ProviderConfig {
    /// 解析 API Key（支持 `env:VAR_NAME`）。
    pub fn resolved_api_key(&self) -> Option<String> {
        match self.api_key.as_deref() {
            Some(v) if v.starts_with("env:") => {
                std::env::var(&v[4..]).ok().filter(|s| !s.is_empty())
            }
            Some(v) if !v.is_empty() => Some(v.to_string()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 默认使用的 provider name
    #[serde(default = "default_provider")]
    pub default_provider: String,
    #[serde(default)]
    pub providers: Vec<ProviderConfig>,
    /// 上下文窗口（token 近似预算），用于历史截断
    #[serde(default = "default_context_window")]
    pub context_window: usize,
    /// 单次用户输入允许的最大工具调用轮数
    #[serde(default = "default_max_tool_rounds")]
    pub max_tool_rounds: usize,
    /// 追加到系统提示词的自定义内容
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// 项目工作目录（绝对路径）；为空时使用进程启动目录
    #[serde(default)]
    pub workspace: Option<String>,
}

fn default_provider() -> String {
    "openai".to_string()
}

fn default_context_window() -> usize {
    128_000
}

fn default_max_tool_rounds() -> usize {
    25
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_provider: default_provider(),
            providers: Vec::new(),
            context_window: default_context_window(),
            max_tool_rounds: default_max_tool_rounds(),
            system_prompt: None,
            workspace: None,
        }
    }
}

impl Config {
    /// 加载配置；文件不存在时写入模板并返回。
    pub fn load() -> Result<Config> {
        let path = paths::config_path();
        if !path.exists() {
            let cfg = Config::template();
            if let Err(e) = cfg.save() {
                tracing::warn!("无法写入默认配置文件 {}: {e}", path.display());
            } else {
                tracing::info!("已生成默认配置文件 {}", path.display());
            }
            return Ok(cfg);
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|e| Error::Config(format!("read {}: {e}", path.display())))?;
        toml::from_str(&text).map_err(|e| Error::Config(format!("parse {}: {e}", path.display())))
    }

    pub fn save(&self) -> Result<()> {
        let path = paths::config_path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| Error::Config(e.to_string()))?;
        }
        let text = toml::to_string_pretty(self).map_err(|e| Error::Config(e.to_string()))?;
        std::fs::write(&path, text)
            .map_err(|e| Error::Config(format!("write {}: {e}", path.display())))
    }

    /// 首次运行写入的模板（含常见 provider 示例）。
    pub fn template() -> Config {
        Config {
            providers: vec![
                ProviderConfig {
                    name: "openai".into(),
                    base_url: "https://api.openai.com/v1".into(),
                    api_key: Some("env:OPENAI_API_KEY".into()),
                    model: "gpt-5".into(),
                    wire_api: WireApi::Responses,
                    headers: HashMap::new(),
                    max_tokens: None,
                    temperature: None,
                },
                ProviderConfig {
                    name: "deepseek".into(),
                    base_url: "https://api.deepseek.com/v1".into(),
                    api_key: Some("env:DEEPSEEK_API_KEY".into()),
                    model: "deepseek-chat".into(),
                    wire_api: WireApi::Chat,
                    headers: HashMap::new(),
                    max_tokens: None,
                    temperature: None,
                },
                ProviderConfig {
                    name: "ollama".into(),
                    base_url: "http://localhost:11434/v1".into(),
                    api_key: None,
                    model: "qwen3:8b".into(),
                    wire_api: WireApi::Chat,
                    headers: HashMap::new(),
                    max_tokens: None,
                    temperature: None,
                },
            ],
            ..Config::default()
        }
    }

    /// 解析要使用的 provider（name 为空时依次取 `SEN_PROVIDER`、`default_provider`），
    /// 并应用环境变量覆盖。
    pub fn resolve_provider(&self, name: Option<&str>) -> Result<ProviderConfig> {
        let want = name
            .map(|s| s.to_string())
            .or_else(|| std::env::var("SEN_PROVIDER").ok().filter(|s| !s.is_empty()))
            .unwrap_or_else(|| self.default_provider.clone());
        let mut p = self
            .providers
            .iter()
            .find(|p| p.name == want)
            .cloned()
            .ok_or_else(|| {
                let available = self
                    .providers
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                Error::Config(format!(
                    "provider `{want}` not found (available: {available}); 检查 ~/.sen-agent/config.toml"
                ))
            })?;
        if let Ok(v) = std::env::var("SEN_BASE_URL") {
            if !v.is_empty() {
                p.base_url = v;
            }
        }
        if let Ok(v) = std::env::var("SEN_API_KEY") {
            if !v.is_empty() {
                p.api_key = Some(v);
            }
        }
        if let Ok(v) = std::env::var("SEN_MODEL") {
            if !v.is_empty() {
                p.model = v;
            }
        }
        if let Ok(v) = std::env::var("SEN_WIRE_API") {
            if !v.is_empty() {
                p.wire_api = WireApi::parse(&v)?;
            }
        }
        Ok(p)
    }

    /// 供历史截断使用的 token 预算（为输出预留空间）。
    pub fn context_budget(&self) -> usize {
        self.context_window.saturating_sub(4_096).max(1_024)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_api_parses() {
        assert_eq!(WireApi::parse("chat").unwrap(), WireApi::Chat);
        assert_eq!(WireApi::parse("Responses").unwrap(), WireApi::Responses);
        assert!(WireApi::parse("nope").is_err());
    }

    #[test]
    fn template_roundtrips_toml() {
        let t = Config::template();
        let text = toml::to_string_pretty(&t).unwrap();
        let back: Config = toml::from_str(&text).unwrap();
        assert_eq!(back.providers.len(), 3);
        assert_eq!(back.providers[0].wire_api, WireApi::Responses);
    }

    #[test]
    fn api_key_env_resolution() {
        std::env::set_var("SEN_TEST_KEY_XYZ", "sekret");
        let p = ProviderConfig {
            name: "t".into(),
            base_url: "http://x".into(),
            api_key: Some("env:SEN_TEST_KEY_XYZ".into()),
            model: "m".into(),
            wire_api: WireApi::Chat,
            headers: HashMap::new(),
            max_tokens: None,
            temperature: None,
        };
        assert_eq!(p.resolved_api_key().as_deref(), Some("sekret"));
    }
}
