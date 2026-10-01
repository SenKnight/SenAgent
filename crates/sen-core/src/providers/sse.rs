//! SSE（Server-Sent Events）解析。
//!
//! chat 与 responses 两种协议的流式响应都基于 SSE；本模块把字节流解析为
//! [`SseEvent`]（可选 event 名 + data 载荷），由各协议适配器进一步归一化。

use bytes::Bytes;
use futures_util::{Stream, StreamExt};

use crate::error::{Error, Result};

#[derive(Debug, Clone, Default)]
pub struct SseEvent {
    /// `event:` 字段（responses 协议使用类型化事件；chat 协议通常为空）
    pub event: Option<String>,
    /// `data:` 载荷（多行以 `\n` 相连）
    pub data: String,
}

/// 把字节流解析为 SSE 事件流。
///
/// 处理规则（兼容 spec 常用子集）：空行分帧；`event:` / `data:` 字段；
/// 多行 data 以 `\n` 相连；忽略注释行与 `id:` / `retry:` 字段；兼容 CRLF。
pub fn sse_stream<S>(stream: S) -> impl Stream<Item = Result<SseEvent>> + Send
where
    S: Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Send + 'static,
{
    async_stream::stream! {
        futures_util::pin_mut!(stream);
        let mut buf: Vec<u8> = Vec::new();
        let mut cur_event: Option<String> = None;
        let mut cur_data = String::new();
        let mut has_field = false;

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => buf.extend_from_slice(&bytes),
                Err(e) => {
                    yield Err(Error::Http(e));
                    return;
                }
            }
            while let Some(line) = pop_line(&mut buf) {
                if line.is_empty() {
                    if has_field || cur_event.is_some() {
                        yield Ok(SseEvent {
                            event: cur_event.take(),
                            data: std::mem::take(&mut cur_data),
                        });
                        has_field = false;
                    }
                } else if let Some(rest) = line.strip_prefix("data:") {
                    if !cur_data.is_empty() {
                        cur_data.push('\n');
                    }
                    cur_data.push_str(rest.strip_prefix(' ').unwrap_or(rest));
                    has_field = true;
                } else if let Some(rest) = line.strip_prefix("event:") {
                    cur_event = Some(rest.trim_start().to_string());
                } else if line.starts_with(':') {
                    // 注释行，忽略
                }
                // 其余字段（id: / retry:）忽略
            }
        }
        // 流结束时冲刷未分帧的最后一个事件
        if has_field || cur_event.is_some() {
            yield Ok(SseEvent {
                event: cur_event.take(),
                data: std::mem::take(&mut cur_data),
            });
        }
    }
}

fn pop_line(buf: &mut Vec<u8>) -> Option<String> {
    let pos = buf.iter().position(|b| *b == b'\n')?;
    let mut line: Vec<u8> = buf.drain(..=pos).collect();
    line.pop(); // 去掉 '\n'
    if line.last() == Some(&b'\r') {
        line.pop();
    }
    Some(String::from_utf8_lossy(&line).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::stream;

    type Chunk = std::result::Result<Bytes, reqwest::Error>;

    async fn collect(chunks: Vec<Chunk>) -> Vec<SseEvent> {
        let s = sse_stream(stream::iter(chunks));
        futures_util::pin_mut!(s);
        let mut out = Vec::new();
        while let Some(e) = s.next().await {
            out.push(e.unwrap());
        }
        out
    }

    #[tokio::test]
    async fn parses_events_split_across_chunks() {
        let chunks: Vec<Chunk> = vec![
            Ok(Bytes::from("event: response.output_text.delta\n")),
            Ok(Bytes::from("data: {\"delta\":\"a\"}\n\ndata: {\"delta\":\"b\"}\n\n")),
            Ok(Bytes::from("data: [DONE]\n\n")),
        ];
        let events = collect(chunks).await;
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].event.as_deref(), Some("response.output_text.delta"));
        assert_eq!(events[0].data, "{\"delta\":\"a\"}");
        assert_eq!(events[1].event, None);
        assert_eq!(events[1].data, "{\"delta\":\"b\"}");
        assert_eq!(events[2].data, "[DONE]");
    }

    #[tokio::test]
    async fn handles_crlf_comments_and_utf8_split() {
        let mut chunks: Vec<Chunk> = vec![
            Ok(Bytes::from(": keep-alive comment\r\n")),
            Ok(Bytes::from("data: 你好".as_bytes()[..8].to_vec())),
        ];
        // 在 UTF-8 多字节字符中间切断
        chunks.push(Ok(Bytes::from("世界\r\n\r\n")));
        let events = collect(chunks).await;
        assert_eq!(events.len(), 1);
        assert!(events[0].data.contains("世界") || events[0].data.contains('\u{fffd}'));
    }
}
