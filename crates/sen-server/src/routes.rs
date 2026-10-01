//! REST 端点：会话 CRUD、运行时信息。

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use sen_core::Session;

use crate::AppState;

/// API 错误：统一返回 `{ "error": "..." }`。
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn not_found(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: msg.into(),
        }
    }
}

impl From<sen_core::Error> for ApiError {
    fn from(e: sen_core::Error) -> Self {
        let message = e.to_string();
        let status = if message.contains("不存在") || message.contains("not found") {
            StatusCode::NOT_FOUND
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };
        Self { status, message }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}

pub async fn health(State(st): State<AppState>) -> Json<Value> {
    Json(json!({
        "ok": true,
        "model": st.agent.provider().model(),
        "wire_api": st.agent.provider().wire_api(),
        "tools": st.agent.tools().names(),
    }))
}

/// 运行时信息（不含任何密钥）。
pub async fn config_info(State(st): State<AppState>) -> Json<Value> {
    Json(json!({
        "provider": st.agent.provider().name(),
        "model": st.agent.provider().model(),
        "wire_api": st.agent.provider().wire_api(),
        "cwd": st.agent.cwd().display().to_string(),
        "skills": st.agent.skills().scan().iter().map(|s| json!({
            "name": s.name,
            "description": s.description,
            "group": s.group,
        })).collect::<Vec<_>>(),
    }))
}

pub async fn list_sessions(State(st): State<AppState>) -> Result<Json<Vec<Session>>, ApiError> {
    Ok(Json(st.agent.store().list_sessions()?))
}

#[derive(Deserialize, Default)]
pub struct CreateSessionReq {
    #[serde(default)]
    pub title: Option<String>,
}

pub async fn create_session(
    State(st): State<AppState>,
    body: Option<Json<CreateSessionReq>>,
) -> Result<Json<Session>, ApiError> {
    let title = body.and_then(|b| b.0.title).unwrap_or_default();
    Ok(Json(st.agent.store().create_session(&title)?))
}

pub async fn get_session(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let session = st
        .agent
        .store()
        .get_session(&id)?
        .ok_or_else(|| ApiError::not_found(format!("会话不存在: {id}")))?;
    let messages = st.agent.store().load_messages(&id)?;
    Ok(Json(json!({ "session": session, "messages": messages })))
}

#[derive(Deserialize)]
pub struct RenameReq {
    pub title: String,
}

pub async fn rename_session(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<RenameReq>,
) -> Result<Json<Session>, ApiError> {
    st.agent
        .store()
        .get_session(&id)?
        .ok_or_else(|| ApiError::not_found(format!("会话不存在: {id}")))?;
    st.agent.store().rename_session(&id, &req.title)?;
    let updated = st
        .agent
        .store()
        .get_session(&id)?
        .ok_or_else(|| ApiError::not_found(format!("会话不存在: {id}")))?;
    Ok(Json(updated))
}

pub async fn delete_session(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    st.agent.store().delete_session(&id)?;
    Ok(Json(json!({ "ok": true })))
}
