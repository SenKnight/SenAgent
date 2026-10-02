//! SenAgent Web 服务（axum）：WebSocket 聊天 + REST 会话管理 + 前端静态托管。
//!
//! `sen serve`（CLI）与桌面端（Tauri 内嵌同进程）共用本 crate；
//! WS 上传输的消息即 [`sen_core::AgentEvent`] 的 JSON（`type` 字段区分类型）。

mod routes;
mod ws;

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use anyhow::Context;
use axum::{
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

use sen_core::Agent;

/// 内嵌前端资源接口。
///
/// CLI 把前端构建产物编译进二进制（`rust-embed`）后通过本接口提供页面：
/// 资源随可执行文件分发，无需外部目录，单文件即可运行 Web UI。
pub trait WebAssets: Send + Sync + 'static {
    /// 按相对路径（不含前导 `/`）读取资源，返回（内容, Content-Type）。
    fn get(&self, path: &str) -> Option<(Vec<u8>, String)>;
}

/// 服务运行参数。
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    /// 生产模式下托管的前端构建产物目录（`frontend/dist`）
    pub static_dir: Option<PathBuf>,
    /// 内嵌前端资源（优先级高于 `static_dir`，供单文件 CLI 使用）
    pub assets: Option<Arc<dyn WebAssets>>,
}

/// 共享状态：服务持有当前 Agent；配置更新时原子替换。
///
/// WS 每收到一轮消息都取最新快照，因此页面保存配置后无需重启即生效。
#[derive(Clone)]
pub struct AppState {
    agent: Arc<RwLock<Agent>>,
}

impl AppState {
    pub fn new(agent: Agent) -> Self {
        Self {
            agent: Arc::new(RwLock::new(agent)),
        }
    }

    /// 取当前 Agent 快照（内部字段均为 Arc，克隆廉价）。
    pub fn agent(&self) -> Agent {
        self.agent
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// 配置热更新：在集合内替换为新构建的 Agent。
    pub fn replace_agent(&self, agent: Agent) {
        *self.agent.write().unwrap_or_else(|e| e.into_inner()) = agent;
    }
}

/// 构建路由（供 `serve` 与后续集成测试复用）。
pub fn build_router(state: AppState, static_dir: Option<PathBuf>) -> Router {
    build_router_with_assets(state, static_dir, None)
}

/// 构建路由：前端资源支持「目录托管」（`static_dir`）或「内嵌资源」（`assets`）。
pub fn build_router_with_assets(
    state: AppState,
    static_dir: Option<PathBuf>,
    assets: Option<Arc<dyn WebAssets>>,
) -> Router {
    let mut app = Router::new()
        .route("/api/health", get(routes::health))
        .route("/api/config", get(routes::config_info))
        .route(
            "/api/settings",
            get(routes::get_settings).put(routes::put_settings),
        )
        .route(
            "/api/sessions",
            get(routes::list_sessions).post(routes::create_session),
        )
        .route(
            "/api/sessions/{id}",
            get(routes::get_session)
                .patch(routes::rename_session)
                .delete(routes::delete_session),
        )
        .route("/api/ws", get(ws::ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    if let Some(assets) = assets {
        app = app.fallback(move |uri: Uri| {
            let assets = assets.clone();
            async move { serve_embedded(&*assets, uri.path()) }
        });
    } else if let Some(dir) = static_dir.filter(|d| d.is_dir()) {
        let index = dir.join("index.html");
        app = app.fallback_service(ServeDir::new(&dir).fallback(ServeFile::new(index)));
    }
    app
}

/// 内嵌资源响应：精确命中返回文件；非文件路径回退 `index.html`（SPA 客户端路由）。
fn serve_embedded(assets: &dyn WebAssets, uri_path: &str) -> Response {
    let rel = uri_path.trim_start_matches('/');
    let rel = if rel.is_empty() { "index.html" } else { rel };
    if let Some((data, mime)) = assets.get(rel) {
        return ([(header::CONTENT_TYPE, mime)], data).into_response();
    }
    // 形似文件（带扩展名）却未命中 → 404；否则交给前端路由处理
    let looks_like_file = rel.rsplit('/').next().is_some_and(|f| f.contains('.'));
    if !looks_like_file {
        if let Some((data, mime)) = assets.get("index.html") {
            return ([(header::CONTENT_TYPE, mime)], data).into_response();
        }
    }
    StatusCode::NOT_FOUND.into_response()
}

/// 启动 HTTP 服务（阻塞直到进程退出）。
pub async fn serve(agent: Agent, cfg: ServerConfig) -> anyhow::Result<()> {
    let addr = format!("{}:{}", cfg.host, cfg.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("无法绑定地址 {addr}"))?;
    tracing::info!("SenAgent server listening on http://{addr}");
    serve_on_with_assets(listener, agent, cfg.static_dir, cfg.assets).await
}

/// 在已绑定的 listener 上运行服务（仅目录托管模式）。
///
/// 桌面端（Tauri 内嵌）先用 `TcpListener::bind(("127.0.0.1", 0))` 拿到随机端口，
/// 再把 listener 交给本函数，从而避免端口冲突。
pub async fn serve_on(
    listener: tokio::net::TcpListener,
    agent: Agent,
    static_dir: Option<PathBuf>,
) -> anyhow::Result<()> {
    serve_on_with_assets(listener, agent, static_dir, None).await
}

/// 在已绑定的 listener 上运行服务（可携带内嵌前端资源）。
pub async fn serve_on_with_assets(
    listener: tokio::net::TcpListener,
    agent: Agent,
    static_dir: Option<PathBuf>,
    assets: Option<Arc<dyn WebAssets>>,
) -> anyhow::Result<()> {
    let app = build_router_with_assets(AppState::new(agent), static_dir, assets);
    axum::serve(listener, app)
        .await
        .context("HTTP 服务异常退出")?;
    Ok(())
}
