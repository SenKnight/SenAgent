//! SenAgent Web 服务（axum）：WebSocket 聊天 + REST 会话管理 + 前端静态托管。
//!
//! `sen serve`（CLI）与桌面端（Tauri 内嵌同进程）共用本 crate；
//! WS 上传输的消息即 [`sen_core::AgentEvent`] 的 JSON（`type` 字段区分类型）。

mod routes;
mod ws;

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use anyhow::Context;
use axum::{routing::get, Router};
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

use sen_core::Agent;

/// 服务运行参数。
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    /// 生产模式下托管的前端构建产物目录（`frontend/dist`）
    pub static_dir: Option<PathBuf>,
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

    if let Some(dir) = static_dir.filter(|d| d.is_dir()) {
        let index = dir.join("index.html");
        app = app.fallback_service(ServeDir::new(&dir).fallback(ServeFile::new(index)));
    }
    app
}

/// 启动 HTTP 服务（阻塞直到进程退出）。
pub async fn serve(agent: Agent, cfg: ServerConfig) -> anyhow::Result<()> {
    let addr = format!("{}:{}", cfg.host, cfg.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("无法绑定地址 {addr}"))?;
    tracing::info!("SenAgent server listening on http://{addr}");
    serve_on(listener, agent, cfg.static_dir).await
}

/// 在已绑定的 listener 上运行服务。
///
/// 桌面端（Tauri 内嵌）先用 `TcpListener::bind(("127.0.0.1", 0))` 拿到随机端口，
/// 再把 listener 交给本函数，从而避免端口冲突。
pub async fn serve_on(
    listener: tokio::net::TcpListener,
    agent: Agent,
    static_dir: Option<PathBuf>,
) -> anyhow::Result<()> {
    let app = build_router(AppState::new(agent), static_dir);
    axum::serve(listener, app)
        .await
        .context("HTTP 服务异常退出")?;
    Ok(())
}
