//! REST 端点：会话 CRUD、运行时信息、配置读写（页面交互式设置）。

use std::collections::HashSet;

use axum::{
    extract::{Path, Query, State},
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

/// 判断请求 `Origin` 与请求 `Host` 是否同源。
///
/// 比较「主机名」而非完整 authority，并把回环地址（`localhost` / `127.0.0.1` / `::1`）
/// 视为等价：反向代理（如 Vite dev proxy 的 `changeOrigin`）会把 `Host` 改写为目标地址，
/// 而浏览器发送的 `Origin` 仍是页面地址，二者端口/主机写法可能不同但属同源。
fn same_site(origin: &str, host: &str) -> bool {
    let origin_host = origin
        .split("://")
        .nth(1)
        .unwrap_or(origin)
        .split('/')
        .next()
        .unwrap_or("");
    if origin_host.is_empty() {
        return false;
    }
    if origin_host.eq_ignore_ascii_case(host) {
        return true;
    }
    let oh = host_name(origin_host);
    let hh = host_name(host);
    oh.eq_ignore_ascii_case(&hh) || (is_loopback(&oh) && is_loopback(&hh))
}

/// 取 authority 的主机名部分（剥离端口，兼容 `[::1]:port` 形式）。
fn host_name(authority: &str) -> String {
    let a = authority.trim();
    if let Some(rest) = a.strip_prefix('[') {
        if let Some((h, _)) = rest.split_once(']') {
            return h.to_string();
        }
    }
    match a.rsplit_once(':') {
        Some((h, p)) if !h.is_empty() && p.chars().all(|c| c.is_ascii_digit()) => h.to_string(),
        _ => a.to_string(),
    }
}

/// 常见回环主机名。
fn is_loopback(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1")
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
    Json(runtime_json(&st.agent()))
}

/// Agent → 运行时信息 JSON（不含密钥）。
fn runtime_json(agent: &Agent) -> Value {
    json!({
        "provider": agent.provider().name(),
        "model": agent.provider().model(),
        "wire_api": agent.provider().wire_api(),
        "cwd": agent.cwd().display().to_string(),
        "home": sen_core::paths::home_dir().display().to_string(),
        "skills": agent.skills().scan().iter().map(|s| json!({
            "name": s.name,
            "description": s.description,
            "group": s.group,
        })).collect::<Vec<_>>(),
    })
}

pub async fn list_sessions(State(st): State<AppState>) -> Result<Json<Vec<Session>>, ApiError> {
    Ok(Json(st.agent().store().list_sessions()?))
}

#[derive(Deserialize, Default)]
pub struct CreateSessionReq {
    #[serde(default)]
    pub title: Option<String>,
    /// 所属项目目录（缺省 = 当前工作目录）。
    #[serde(default)]
    pub workspace: Option<String>,
}

pub async fn create_session(
    State(st): State<AppState>,
    body: Option<Json<CreateSessionReq>>,
) -> Result<Json<Session>, ApiError> {
    let req = body.map(|b| b.0).unwrap_or_default();
    let title = req.title.unwrap_or_default();
    let workspace = req
        .workspace
        .filter(|w| !w.trim().is_empty())
        .unwrap_or_else(|| st.agent().cwd().display().to_string());
    Ok(Json(st.agent().store().create_session(&title, Some(&workspace))?))
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
    // 轻量 CSRF 防护：浏览器跨源请求会携带 Origin，其主机必须与请求 Host 同源。
    // 比较主机名且兼容回环地址，避免反向代理改写 Host 后误拒同源保存。
    if let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) {
        let host = headers
            .get(header::HOST)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        if !same_site(origin, host) {
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
        // 保留当前工作目录设置（由 /api/workspace 单独维护）
        workspace: current.workspace.clone(),
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

/// 自动发现 provider 可用模型：`POST /api/models`。
///
/// 请求体 `{ name?, base_url, api_key?, wire_api? }`，用于设置页在保存前
/// 探测模型列表；`api_key` 缺省/为空时，若同名 provider 已配置则回退使用其已存密钥。
#[derive(Deserialize)]
pub struct ListModelsReq {
    #[serde(default)]
    pub name: Option<String>,
    pub base_url: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub wire_api: Option<String>,
}

pub async fn list_models(
    State(st): State<AppState>,
    Json(req): Json<ListModelsReq>,
) -> Result<Json<Value>, ApiError> {
    let base_url = req.base_url.trim().to_string();
    if base_url.is_empty() {
        return Err(ApiError::bad_request("base_url 不能为空"));
    }
    let wire_api = match req.wire_api.as_deref().filter(|s| !s.is_empty()) {
        Some(s) => WireApi::parse(s).map_err(|e| ApiError::bad_request(e.to_string()))?,
        None => WireApi::Chat,
    };
    let name = req.name.clone().unwrap_or_default();
    let saved = if name.is_empty() {
        None
    } else {
        st.agent()
            .config()
            .providers
            .iter()
            .find(|p| p.name == name)
            .cloned()
    };
    let api_key = req
        .api_key
        .clone()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| saved.as_ref().and_then(|p| p.api_key.clone()));
    let cfg = ProviderConfig {
        name: if name.is_empty() {
            "discover".to_string()
        } else {
            name
        },
        base_url,
        api_key,
        model: String::new(),
        wire_api,
        headers: saved.map(|p| p.headers).unwrap_or_default(),
        max_tokens: None,
        temperature: None,
    };
    let models = sen_core::providers::list_models(&cfg)
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(json!({ "models": models })))
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

// ── 项目文件树（只读预览）────────────────────────────────────────────

#[derive(Deserialize)]
pub struct FileQuery {
    #[serde(default)]
    pub path: Option<String>,
}

/// 列出工作目录内某目录的直接子项（只读）：`GET /api/files/tree?path=<rel>`。
///
/// 返回 `[{ name, path, is_dir, size, modified_at }]`，`path` 相对工作目录、统一 `/`；
/// 包含隐藏项（如 `.sen-agent`）；单目录条目上限 2000。
pub async fn file_tree(
    State(st): State<AppState>,
    Query(q): Query<FileQuery>,
) -> Result<Json<Value>, ApiError> {
    let cwd = st.agent().cwd().to_path_buf();
    let rel = q.path.unwrap_or_default().trim_matches('/').to_string();
    let dir = resolve_within(&cwd, &rel)?;
    if !dir.is_dir() {
        return Err(ApiError::bad_request("目标不是目录"));
    }
    let rd =
        std::fs::read_dir(&dir).map_err(|e| ApiError::bad_request(format!("读取目录失败: {e}")))?;
    let mut entries: Vec<Value> = Vec::new();
    for ent in rd.flatten().take(2000) {
        let Ok(ft) = ent.file_type() else { continue };
        let name = ent.file_name().to_string_lossy().to_string();
        let child = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        let md = ent.metadata().ok();
        let modified_at = md
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        entries.push(json!({
            "name": name,
            "path": child,
            "is_dir": ft.is_dir(),
            "size": if ft.is_dir() { 0 } else { md.as_ref().map(|m| m.len()).unwrap_or(0) },
            "modified_at": modified_at,
        }));
    }
    // 目录在前，再按名称升序
    entries.sort_by(|a, b| {
        let ad = a["is_dir"].as_bool().unwrap_or(false);
        let bd = b["is_dir"].as_bool().unwrap_or(false);
        bd.cmp(&ad)
            .then_with(|| a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or("")))
    });
    Ok(Json(Value::Array(entries)))
}

/// 读取工作目录内文本文件（只读）：`GET /api/files/content?path=<rel>`。
///
/// 二进制文件、越界路径均返回 400；超过 ~200KB 的内容截断。
pub async fn file_content(
    State(st): State<AppState>,
    Query(q): Query<FileQuery>,
) -> Result<Json<Value>, ApiError> {
    let cwd = st.agent().cwd().to_path_buf();
    let rel = q
        .path
        .map(|p| p.trim_matches('/').to_string())
        .filter(|p| !p.is_empty())
        .ok_or_else(|| ApiError::bad_request("缺少 path 参数"))?;
    let file = resolve_within(&cwd, &rel)?;
    if !file.is_file() {
        return Err(ApiError::bad_request("目标不是文件"));
    }
    let bytes =
        std::fs::read(&file).map_err(|e| ApiError::bad_request(format!("读取文件失败: {e}")))?;
    let content =
        String::from_utf8(bytes).map_err(|_| ApiError::bad_request("二进制文件不支持预览"))?;
    Ok(Json(json!({
        "path": rel,
        "content": sen_core::util::truncate(&content, 200_000),
    })))
}

/// 列出某会话的计划：`GET /api/sessions/{id}/plans`。
pub async fn list_plans(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let plans = st.agent().store().list_plans(&id)?;
    Ok(Json(json!(plans)))
}

// ── 工作目录（项目目录选择）───────────────────────────────────────────

/// 浏览文件系统目录（用于选择项目目录）：`GET /api/fs/dirs?path=<abs>`。
///
/// `path` 缺省 = 用户主目录。返回 `{ path, parent, dirs: [{ name, path }] }`；
/// 仅列出子目录（隐藏项跳过），`parent` 为上级目录绝对路径（根目录为 null）。
pub async fn fs_dirs(Query(q): Query<FileQuery>) -> Result<Json<Value>, ApiError> {
    let raw = q
        .path
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| sen_core::paths::home_dir().display().to_string());
    let dir = std::path::PathBuf::from(&raw);
    if !dir.is_dir() {
        return Err(ApiError::bad_request(format!("目录不存在: {raw}")));
    }
    let dir = dir.canonicalize().unwrap_or(dir);
    let rd =
        std::fs::read_dir(&dir).map_err(|e| ApiError::bad_request(format!("读取目录失败: {e}")))?;
    let mut dirs: Vec<Value> = Vec::new();
    for ent in rd.flatten().take(2000) {
        let Ok(ft) = ent.file_type() else { continue };
        if !ft.is_dir() {
            continue;
        }
        let name = ent.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        dirs.push(json!({ "name": name, "path": ent.path().display().to_string() }));
    }
    dirs.sort_by(|a, b| {
        a["name"]
            .as_str()
            .unwrap_or("")
            .cmp(b["name"].as_str().unwrap_or(""))
    });
    let parent = dir.parent().map(|p| p.display().to_string());
    Ok(Json(json!({
        "path": dir.display().to_string(),
        "parent": parent,
        "dirs": dirs,
    })))
}

#[derive(Deserialize)]
pub struct SetWorkspaceReq {
    pub path: String,
}

/// 切换项目工作目录并持久化：`PUT /api/workspace`。
///
/// 仅接受存在的绝对目录；成功后热替换 Agent 的 cwd（作用于文件树、文件读写与
/// 命令执行），并写入 `config.workspace`，下次启动默认使用。
pub async fn set_workspace(
    State(st): State<AppState>,
    Json(req): Json<SetWorkspaceReq>,
) -> Result<Json<Value>, ApiError> {
    let raw = req.path.trim();
    if raw.is_empty() {
        return Err(ApiError::bad_request("path 不能为空"));
    }
    let dir = std::path::PathBuf::from(raw);
    if !dir.is_dir() {
        return Err(ApiError::bad_request(format!("目录不存在: {raw}")));
    }
    let cwd = dir
        .canonicalize()
        .map_err(|e| ApiError::bad_request(format!("无法解析目录: {e}")))?;

    let agent = st.agent();
    // 持久化到配置（保留其余字段）
    let mut cfg = agent.config().clone();
    cfg.workspace = Some(cwd.display().to_string());
    cfg.save()?;

    st.replace_agent(agent.with_cwd(cwd));
    Ok(Json(runtime_json(&st.agent())))
}

/// 切换某会话所属项目目录并联动当前工作目录：`PUT /api/sessions/{id}/workspace`。
///
/// 用于在已打开会话时切换项目：同步迁移会话归属（侧栏分组随之更新）、
/// 热替换 Agent 的 cwd，并持久化到配置。
#[derive(Deserialize)]
pub struct SetSessionWorkspaceReq {
    pub path: String,
}

pub async fn set_session_workspace(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<SetSessionWorkspaceReq>,
) -> Result<Json<Value>, ApiError> {
    let raw = req.path.trim();
    if raw.is_empty() {
        return Err(ApiError::bad_request("path 不能为空"));
    }
    let dir = std::path::PathBuf::from(raw);
    if !dir.is_dir() {
        return Err(ApiError::bad_request(format!("目录不存在: {raw}")));
    }
    let cwd = dir
        .canonicalize()
        .map_err(|e| ApiError::bad_request(format!("无法解析目录: {e}")))?;

    let agent = st.agent();
    let store = agent.store().clone();
    store
        .get_session(&id)?
        .ok_or_else(|| ApiError::not_found(format!("会话不存在: {id}")))?;
    store.set_session_workspace(&id, &cwd.display().to_string())?;

    let mut cfg = agent.config().clone();
    cfg.workspace = Some(cwd.display().to_string());
    cfg.save()?;

    st.replace_agent(agent.with_cwd(cwd));
    Ok(Json(runtime_json(&st.agent())))
}

/// 解析相对工作目录的路径，并确保 canonical 后仍位于工作目录内（防目录穿越）。
fn resolve_within(cwd: &std::path::Path, rel: &str) -> Result<std::path::PathBuf, ApiError> {
    let base = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
    let joined = if rel.is_empty() {
        base.clone()
    } else {
        base.join(rel)
    };
    let resolved = joined
        .canonicalize()
        .map_err(|e| ApiError::bad_request(format!("路径不存在或不可访问: {e}")))?;
    if !resolved.starts_with(&base) {
        return Err(ApiError::bad_request("路径越界（仅允许工作目录内）"));
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::{host_name, same_site};

    #[test]
    fn same_site_accepts_same_and_loopback() {
        // 同 authority
        assert!(same_site("http://127.0.0.1:8642", "127.0.0.1:8642"));
        assert!(same_site("http://localhost:5173", "localhost:5173"));
        // 反向代理改写 Host 为 127.0.0.1:8642，页面 Origin 为 localhost:5173 → 回环等价
        assert!(same_site("http://localhost:5173", "127.0.0.1:8642"));
        assert!(same_site("http://127.0.0.1:5173", "localhost:8642"));
    }

    #[test]
    fn same_site_rejects_cross_origin() {
        assert!(!same_site("http://evil.com", "127.0.0.1:8642"));
        assert!(!same_site("http://evil.com:80", "localhost:8642"));
        assert!(!same_site("null", "127.0.0.1:8642"));
    }

    #[test]
    fn host_name_strips_port() {
        assert_eq!(host_name("127.0.0.1:8642"), "127.0.0.1");
        assert_eq!(host_name("localhost"), "localhost");
        assert_eq!(host_name("[::1]:5173"), "::1");
    }
}
