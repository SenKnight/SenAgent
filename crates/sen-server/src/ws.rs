//! WebSocket 端点：接收客户端聊天消息，流式转发 [`sen_core::AgentEvent`]。
//!
//! 客户端 → 服务端（JSON，`type` 字段区分）：
//! - `{"type":"chat","session_id":"...","content":"..."}` 发起一轮对话
//! - `{"type":"ping"}` 连通性探测（回 `{"type":"pong"}`）
//!
//! 服务端 → 客户端：`AgentEvent` 序列化（`token` / `reasoning` / `tool_call` /
//! `tool_result` / `done` / `error`），连接建立后先发一条 `ready`。

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;

use crate::AppState;

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    /// 发起一轮对话
    Chat { session_id: String, content: String },
    /// 连通性探测
    Ping,
}

pub async fn ws_handler(State(st): State<AppState>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| handle(socket, st))
}

async fn handle(socket: WebSocket, st: AppState) {
    let (mut sender, mut receiver) = socket.split();

    // 首帧：就绪信息（含当前模型）
    {
        let agent = st.agent();
        let ready = format!(
            "{{\"type\":\"ready\",\"model\":{},\"wire_api\":{}}}",
            json_str(agent.provider().model()),
            json_str(agent.provider().wire_api())
        );
        if sender.send(Message::Text(ready.into())).await.is_err() {
            return;
        }
    }

    while let Some(Ok(msg)) = receiver.next().await {
        let Message::Text(text) = msg else {
            if matches!(msg, Message::Close(_)) {
                break;
            }
            continue;
        };
        let client_msg: ClientMessage = match serde_json::from_str(text.as_str()) {
            Ok(m) => m,
            Err(e) => {
                let payload = format!(
                    "{{\"type\":\"error\",\"message\":{}}}",
                    json_str(&format!(
                        "消息格式错误（需 {{\"type\":\"chat\",\"session_id\":...,\"content\":...}}）: {e}"
                    ))
                );
                let _ = sender.send(Message::Text(payload.into())).await;
                continue;
            }
        };

        match client_msg {
            ClientMessage::Ping => {
                let _ = sender.send(Message::Text("{\"type\":\"pong\"}".into())).await;
            }
            ClientMessage::Chat {
                session_id,
                content,
            } => {
                if content.trim().is_empty() {
                    continue;
                }
                // 每轮取最新 Agent 快照：页面保存配置后下一轮对话即生效。
                let agent = st.agent();
                match agent.store().get_session(&session_id) {
                    Ok(Some(_)) => {}
                    Ok(None) => {
                        let payload = format!(
                            "{{\"type\":\"error\",\"message\":{}}}",
                            json_str(&format!("会话不存在: {session_id}"))
                        );
                        let _ = sender.send(Message::Text(payload.into())).await;
                        continue;
                    }
                    Err(e) => {
                        let payload = format!(
                            "{{\"type\":\"error\",\"message\":{}}}",
                            json_str(&format!("读取会话失败: {e}"))
                        );
                        let _ = sender.send(Message::Text(payload.into())).await;
                        continue;
                    }
                }

                // 同一连接内串行处理：本轮事件流结束（done/error）后再读下一条消息。
                // 客户端断开（send 失败）会 drop 事件流，即取消本轮生成。
                let stream = agent.run_turn(&session_id, &content);
                futures_util::pin_mut!(stream);
                let mut closed = false;
                while let Some(ev) = stream.next().await {
                    let payload = serde_json::to_string(&ev).unwrap_or_else(|_| {
                        "{\"type\":\"error\",\"message\":\"事件序列化失败\"}".to_string()
                    });
                    if sender.send(Message::Text(payload.into())).await.is_err() {
                        closed = true;
                        break;
                    }
                    if ev.is_terminal() {
                        break;
                    }
                }
                if closed {
                    break;
                }
            }
        }
    }
}

/// 序列化为 JSON 字符串字面量（含转义）。
fn json_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}
