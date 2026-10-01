//! REST 端点：会话 CRUD、运行时信息、配置读写（页面交互式设置）。

use std::collections::HashSet;

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use sen_core::{Agent, Config, ProviderConfig, Session, WireApi};

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

    fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
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
    let agent = st.agent();
    Json(json!({
        "ok": true,
        "model": agent.provider().model(),
        "wire_api": agent.provider().wire_api(),
        "tools": agent.tools().names(),
    }))
}

/// 运行时信息（不含任何密钥）。
pub async fn config_info(State(st): State<AppState>) -> Json<Value> {
    let agent = st.agent();
    Json(json!({
        "provider": agent.provider().name(),
        "model": agent.provider().model(),
        "wire_api": agent.provider().wire_api(),
        "cwd": agent.cwd().display().to_string(),
        "skills": agent.skills().scan().iter().map(|s| json!({
            "name": s.name,
            "description": s.description,
            "group": s.group,
        })).collect::<Vec<_>>(),
    }))
}

pub async fn list_sessions(State(st): State<AppState>) -> Result<Json<Vec<Session>>, ApiError> {
    Ok(Json(st.agent().store().list_sessions()?))
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
    Ok(Json(st.agent().store().create_session(&title)?))
}

pub async fn get_session(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let store = st.agent().store().clone();
    let session = store
        .get_session(&id)?
        .ok_or_else(|| ApiError::not_found(format!("会话不存在: {id}")))?;
    let messages = store.load_messages(&id)?;
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
    let store = st.agent().store().clone();
    store
        .get_session(&id)?
        .ok_or_else(|| ApiError::not_found(format!("会话不存在: {id}")))?;
    store.rename_session(&id, &req.title)?;
    let updated = store
        .get_session(&id)?
        .ok_or_else(|| ApiError::not_found(format!("会话不存在: {id}")))?;
    Ok(Json(updated))
}

pub async fn delete_session(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    st.agent().store().delete_session(&id)?;
    Ok(Json(json!({ "ok": true })))
}

// ── 配置读写（页面交互式设置，保存后热生效）────────────────────────────

/// 可编辑配置（GET /api/settings）。
///
/// `api_key`：`env:VAR` 引用原样回显；明文密钥不回显（`api_key` 为 null、
/// `api_key_set` 为 true）；PUT 时字段缺失/null 表示保留原值，空串表示清除。
pub async fn get_settings(State(st): State<AppState>) -> Json<Value> {
    let agent = st.agent();
    Json(settings_json(agent.config()))
}

#[derive(Deserialize)]
pub struct DraftSettings {
    pub default_provider: String,
    #[serde(default = "default_context_window")]
    pub context_window: usize,
    #[serde(default = "default_max_tool_rounds")]
    pub max_tool_rounds: usize,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub providers: Vec<DraftProvider>,
}

#[derive(Deserialize)]
pub struct DraftProvider {
    pub name: String,
    pub base_url: String,
    pub model: String,
    #[serde(default = "default_wire_api")]
    pub wire_api: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub temperature: Option<f32>,
}

fn default_context_window() -> usize {
    128_000
}

fn default_max_tool_rounds() -> usize {
    25
}

fn default_wire_api() -> String {
    "chat".to_string()
}

/// 保存配置：校验 → 构建新 Agent（可解析性验证）→ 落盘 → 原子替换（即时生效）。
pub async fn put_settings(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(draft): Json<DraftSettings>,
) -> Result<Json<Value>, ApiError> {
    // 轻量 CSRF 防护：浏览器跨源请求会携带 Origin，其地址必须与请求 Host 一致。
    if let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) {
        let host = headers
            .get(header::HOST)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        if !origin.ends_with(host) {
            return Err(ApiError {
                status: StatusCode::FORBIDDEN,
                message: "Origin 校验失败（仅允许同源页面修改配置）".to_string(),
            });
        }
    }

    if draft.providers.is_empty() {
        return Err(ApiError::bad_request("至少需要一个 provider"));
    }
    let current = st.agent().config().clone();

    let mut seen: HashSet<String> = HashSet::new();
    let mut providers = Vec::with_capacity(draft.providers.len());
    for d in &draft.providers {
        let name = d.name.trim();
        if name.is_empty() {
            return Err(ApiError::bad_request("provider 名称不能为空"));
        }
        if !seen.insert(name.to_string()) {
            return Err(ApiError::bad_request(format!("provider 名称重复: {name}")));
        }
        if d.base_url.trim().is_empty() {
            return Err(ApiError::bad_request(format!("`{name}` 的 Base URL 不能为空")));
        }
        if d.model.trim().is_empty() {
            return Err(ApiError::bad_request(format!("`{name}` 的模型不能为空")));
        }
        let wire_api =
            WireApi::parse(&d.wire_api).map_err(|e| ApiError::bad_request(e.to_string()))?;

        let old = current.providers.iter().find(|p| p.name == name);
        let api_key = match d.api_key.as_deref() {
            None => old.and_then(|p| p.api_key.clone()),
            Some("") => None,
            Some(v) => Some(v.to_string()),
        };
        providers.push(ProviderConfig {
            name: name.to_string(),
            base_url: d.base_url.trim().to_string(),
            api_key,
            model: d.model.trim().to_string(),
            wire_api,
            headers: old.map(|p| p.headers.clone()).unwrap_or_default(),
            max_tokens: d.max_tokens,
            temperature: d.temperature,
        });
    }

    if !seen.contains(draft.default_provider.trim()) {
        return Err(ApiError::bad_request(format!(
            "默认 provider `{}` 不在列表中",
            draft.default_provider
        )));
    }

    let new_config = Config {
        default_provider: draft.default_provider.trim().to_string(),
        providers,
        context_window: draft.context_window.clamp(1_024, 10_000_000),
        max_tool_rounds: draft.max_tool_rounds.clamp(1, 200),
        system_prompt: draft.system_prompt.filter(|s| !s.trim().is_empty()),
    };

    // 先构建新 Agent 验证配置可解析（如 provider 存在、字段合法）；
    // 成功后再落盘并原子替换，失败则磁盘与运行态都保持原样。
    let cwd = st.agent().cwd().to_path_buf();
    let new_agent = Agent::from_config(new_config.clone(), Some(&new_config.default_provider), cwd)
        .map_err(|e| ApiError::bad_request(format!("配置无法生效: {e}")))?;
    new_config.save()?;
    st.replace_agent(new_agent);

    Ok(Json(settings_json(&new_config)))
}

/// 配置 → 页面可编辑 JSON（密钥脱敏）。
fn settings_json(cfg: &Config) -> Value {
    json!({
        "config_path": sen_core::paths::config_path().display().to_string(),
        "default_provider": cfg.default_provider,
        "context_window": cfg.context_window,
        "max_tool_rounds": cfg.max_tool_rounds,
        "system_prompt": cfg.system_prompt,
        "providers": cfg
            .providers
            .iter()
            .map(|p| {
                let (api_key, api_key_set) = match p.api_key.as_deref() {
                    Some(k) if k.starts_with("env:") => (Some(k), true),
                    Some(_) => (None, true),
                    None => (None, false),
                };
                json!({
                    "name": p.name,
                    "base_url": p.base_url,
                    "model": p.model,
                    "wire_api": p.wire_api.as_str(),
                    "api_key": api_key,
                    "api_key_set": api_key_set,
                    "max_tokens": p.max_tokens,
                    "temperature": p.temperature,
                })
            })
            .collect::<Vec<_>>(),
    })
}
