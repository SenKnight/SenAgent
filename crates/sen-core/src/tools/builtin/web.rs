//! web 工具：抓取网页并转为纯文本。

use std::sync::OnceLock;
use std::time::Duration;

use async_trait::async_trait;
use regex::Regex;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

use crate::error::{Error, Result};
use crate::tools::{schema_of, Tool};
use crate::util::truncate;

pub struct FetchTool;

#[derive(Deserialize, JsonSchema)]
struct FetchArgs {
    /// 要抓取的 URL（http/https）
    url: String,
    /// 返回内容的最大字符数（默认 30000）
    #[serde(default)]
    max_chars: Option<usize>,
}

#[async_trait]
impl Tool for FetchTool {
    fn name(&self) -> &'static str {
        "fetch"
    }

    fn description(&self) -> &'static str {
        "抓取网页或 HTTP 接口内容并返回文本；HTML 会自动转为纯文本，超长内容自动截断。"
    }

    fn parameters_schema(&self) -> Value {
        schema_of::<FetchArgs>()
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let args: FetchArgs =
            serde_json::from_value(args).map_err(|e| Error::Tool(format!("参数不合法: {e}")))?;
        if !args.url.starts_with("http://") && !args.url.starts_with("https://") {
            return Err(Error::Tool("仅支持 http/https URL".into()));
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("SenAgent/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| Error::Tool(format!("创建 HTTP 客户端失败: {e}")))?;
        let resp = client
            .get(&args.url)
            .send()
            .await
            .map_err(|e| Error::Tool(format!("请求 {} 失败: {e}", args.url)))?;

        let status = resp.status();
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let body = resp
            .text()
            .await
            .map_err(|e| Error::Tool(format!("读取响应失败: {e}")))?;

        if !status.is_success() {
            return Err(Error::Tool(format!(
                "请求 {} 返回 {status}: {}",
                args.url,
                truncate(&body, 300)
            )));
        }

        let head = body.trim_start();
        let head_start = head.get(..16).unwrap_or(head).to_ascii_lowercase();
        let is_html = content_type.contains("text/html")
            || head_start.starts_with("<!doctype")
            || head_start.starts_with("<html");
        let text = if is_html { html_to_text(&body) } else { body };
        let max = args.max_chars.unwrap_or(30_000).clamp(1_000, 200_000);
        let body_out = truncate(&text, max);

        Ok(format!(
            "URL: {}\n状态: {status}\n类型: {}\n\n{body_out}",
            args.url,
            if content_type.is_empty() {
                "unknown"
            } else {
                &content_type
            }
        ))
    }
}

/// HTML → 纯文本：去除 script/style、块级标签转行、剥离其余标签、解码常见实体。
fn html_to_text(html: &str) -> String {
    static SCRIPT: OnceLock<Regex> = OnceLock::new();
    static STYLE: OnceLock<Regex> = OnceLock::new();
    static BLOCK: OnceLock<Regex> = OnceLock::new();
    static TAG: OnceLock<Regex> = OnceLock::new();
    static BLANK: OnceLock<Regex> = OnceLock::new();

    let script = SCRIPT.get_or_init(|| Regex::new(r"(?is)<script[\s\S]*?</script>").unwrap());
    let style = STYLE.get_or_init(|| Regex::new(r"(?is)<style[\s\S]*?</style>").unwrap());
    let block = BLOCK.get_or_init(|| {
        Regex::new(r"(?i)<(br|/p|/div|/li|/ul|/ol|/h[1-6]|/tr|/table|/section|/article|/header|/footer)[^>]*>")
            .unwrap()
    });
    let tag = TAG.get_or_init(|| Regex::new(r"(?s)<[^>]*>").unwrap());
    let blank = BLANK.get_or_init(|| Regex::new(r"\n{3,}").unwrap());

    let s = script.replace_all(html, " ");
    let s = style.replace_all(&s, " ");
    let s = block.replace_all(&s, "\n");
    let s = tag.replace_all(&s, "");
    let s = decode_entities(&s);
    let s = blank.replace_all(&s, "\n\n");

    s.lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn decode_entities(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_to_text_strips_tags_and_scripts() {
        let html = "<html><head><style>body{color:red}</style><script>var a=1;</script></head>\
                    <body><h1>Title</h1><p>Hello &amp; world</p><br><p>Second</p></body></html>";
        let text = html_to_text(html);
        assert!(text.contains("Title"));
        assert!(text.contains("Hello & world"));
        assert!(text.contains("Second"));
        assert!(!text.contains("var a=1"));
        assert!(!text.contains("color:red"));
        assert!(!text.contains("<h1>"));
    }

    #[test]
    fn html_to_text_keeps_plain_text() {
        assert_eq!(html_to_text("just plain text"), "just plain text");
    }
}
