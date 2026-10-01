//! 通用工具函数。

use std::time::{SystemTime, UNIX_EPOCH};

use crate::memory::{Message, Role};

/// 粗略估算 token 数（CJK 字符按 1 token，其他按约 4 字符 1 token）
pub fn estimate_tokens(text: &str) -> usize {
    let mut cjk = 0usize;
    let mut other = 0usize;
    for ch in text.chars() {
        if ('\u{3400}'..='\u{9fff}').contains(&ch) {
            cjk += 1;
        } else {
            other += 1;
        }
    }
    cjk + other / 4 + 1
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn format_ts(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|dt| dt.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default()
}

/// 截断字符串到 max_chars 个字符，超长时追加说明。
pub fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut s: String = text.chars().take(max_chars).collect();
    s.push_str(&format!("\n...(truncated, {} chars total)", text.chars().count()));
    s
}

/// 按 token 预算裁剪历史消息：从最旧的开始丢弃，保留最近的对话。
///
/// 裁剪后不保留开头孤立的 tool 结果（其对应的 assistant tool_calls 已被裁掉）。
pub fn trim_messages(messages: &[Message], budget_tokens: usize) -> Vec<Message> {
    let total: usize = messages.iter().map(message_tokens).sum();
    if total <= budget_tokens {
        return messages.to_vec();
    }
    let mut start = 0usize;
    let mut current = total;
    while start < messages.len() && current > budget_tokens {
        current = current.saturating_sub(message_tokens(&messages[start]));
        start += 1;
    }
    while start < messages.len() && messages[start].role == Role::Tool {
        start += 1;
    }
    messages[start..].to_vec()
}

fn message_tokens(m: &Message) -> usize {
    estimate_tokens(&m.content)
        + m.tool_calls
            .as_ref()
            .map(|tcs| tcs.iter().map(|t| estimate_tokens(&t.arguments) + 8).sum())
            .unwrap_or(0)
        + 8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_keeps_short_text() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_marks_long_text() {
        let t = truncate(&"a".repeat(100), 10);
        assert!(t.starts_with(&"a".repeat(10)));
        assert!(t.contains("truncated"));
    }

    #[test]
    fn trim_keeps_recent_messages() {
        let mut messages = Vec::new();
        for i in 0..100 {
            messages.push(Message::user(format!("message number {i} with some padding text")));
        }
        let trimmed = trim_messages(&messages, 60);
        assert!(trimmed.len() < messages.len());
        assert_eq!(trimmed.last().unwrap().content, messages.last().unwrap().content);
    }

    #[test]
    fn trim_under_budget_keeps_all() {
        let messages = vec![Message::user("hi"), Message::assistant("hello", None)];
        assert_eq!(trim_messages(&messages, 10_000).len(), 2);
    }
}
