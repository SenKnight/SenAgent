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
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;

use sen_core::{Agent, TurnMode};

use crate::AppState;

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    /// 发起一轮对话；`mode` 为 `"normal"`（缺省）/ `"plan"`。
    Chat {
        session_id: String,
        content: String,
        #[serde(default)]
        mode: Option<String>,
    },
    /// 执行已确认的计划（计划模式下产出的 `draft` 计划）。
    ExecutePlan { session_id: String, plan_id: String },
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
                mode,
            } => {
                if content.trim().is_empty() {
                    continue;
                }
                let turn_mode = if mode.as_deref() == Some("plan") {
                    TurnMode::Plan
                } else {
                    TurnMode::Normal
                };
                // 每轮取最新 Agent 快照：页面保存配置后下一轮对话即生效。
                if forward_turn(&mut sender, st.agent(), &session_id, &content, turn_mode).await {
                    break;
                }
            }
            ClientMessage::ExecutePlan {
                session_id,
                plan_id,
            } => {
                let agent = st.agent();
                let plan = match agent.store().list_plans(&session_id) {
                    Ok(plans) => plans.into_iter().find(|p| p.id == plan_id),
                    Err(e) => {
                        send_error(&mut sender, &format!("读取计划失败: {e}")).await;
                        continue;
                    }
                };
                let Some(plan) = plan else {
                    send_error(&mut sender, &format!("计划不存在: {plan_id}")).await;
                    continue;
                };
                if let Err(e) = agent.store().set_plan_status(&plan_id, "executed") {
                    tracing::warn!("更新计划状态失败: {e}");
                }
                let input = format!("请按以下计划执行（用户已确认）：\n\n{}", plan.content);
                if forward_turn(&mut sender, agent, &session_id, &input, TurnMode::Normal).await {
                    break;
                }
            }
        }
    }
}

/// 发送一条 error 事件（忽略发送失败）。
async fn send_error(sender: &mut SplitSink<WebSocket, Message>, message: &str) {
    let payload = format!("{{\"type\":\"error\",\"message\":{}}}", json_str(message));
    let _ = sender.send(Message::Text(payload.into())).await;
}

/// 校验会话并转发一轮事件流；返回 `true` 表示连接已断开，需退出读取循环。
async fn forward_turn(
    sender: &mut SplitSink<WebSocket, Message>,
    agent: Agent,
    session_id: &str,
    input: &str,
    mode: TurnMode,
) -> bool {
    // 会话所属项目目录即本轮工作目录（无项目时由 store 归一为用户主目录）。
    let session = match agent.store().get_session(session_id) {
        Ok(Some(s)) => s,
        Ok(None) => {
            send_error(sender, &format!("会话不存在: {session_id}")).await;
            return false;
        }
        Err(e) => {
            send_error(sender, &format!("读取会话失败: {e}")).await;
            return false;
        }
    };
    let agent = if session.workspace.is_empty() {
        agent
    } else {
        agent.with_cwd(std::path::PathBuf::from(&session.workspace))
    };

    // 同一连接内串行处理：本轮事件流结束（done/error）后再读下一条消息。
    // 客户端断开（send 失败）会 drop 事件流，即取消本轮生成。
    let stream = agent.run_turn(session_id, input, mode);
    futures_util::pin_mut!(stream);
    while let Some(ev) = stream.next().await {
        let payload = serde_json::to_string(&ev)
            .unwrap_or_else(|_| "{\"type\":\"error\",\"message\":\"事件序列化失败\"}".to_string());
        if sender.send(Message::Text(payload.into())).await.is_err() {
            return true;
        }
        if ev.is_terminal() {
            break;
        }
    }
    false
}

/// 序列化为 JSON 字符串字面量（含转义）。
fn json_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}
