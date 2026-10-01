//! `/api/settings` 集成测试：密钥脱敏与合并、校验失败不落盘、保存后热替换。
//!
//! 注：单个测试函数内完成全部场景（设置全局环境变量 SEN_AGENT_HOME /
//! SEN_AGENTS_DIR 指向临时目录，避免并行测试互相干扰与污染真实数据目录）。

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

use sen_core::{Agent, Config};
use sen_server::{build_router, AppState};

fn setup() -> (tempfile::TempDir, AppState, axum::Router) {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("SEN_AGENT_HOME", dir.path().join("home"));
    std::env::set_var("SEN_AGENTS_DIR", dir.path().join("agents"));

    let config = Config::template();
    config.save().unwrap();
    let agent = Agent::from_config(config, Some("ollama"), dir.path().to_path_buf()).unwrap();
    let state = AppState::new(agent);
    let app = build_router(state.clone(), None);
    (dir, state, app)
}

async fn body_json(resp: axum::response::Response) -> (StatusCode, Value) {
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v = serde_json::from_slice(&bytes).unwrap();
    (status, v)
}

async fn get_settings(app: &axum::Router) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/settings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    body_json(resp).await
}

async fn put_settings(app: &axum::Router, payload: Value) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/settings")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    body_json(resp).await
}

#[tokio::test]
async fn settings_roundtrip_mask_merge_and_hot_apply() {
    let (_dir, state, app) = setup();

    // ── GET：env: 引用原样回显 ────────────────────────────────────────
    let (status, v) = get_settings(&app).await;
    assert_eq!(status, StatusCode::OK);
    let providers = v["providers"].as_array().unwrap();
    assert_eq!(providers.len(), 3);
    let openai = providers.iter().find(|p| p["name"] == "openai").unwrap();
    assert_eq!(openai["api_key"], "env:OPENAI_API_KEY");
    assert_eq!(openai["api_key_set"], true);
    let ollama = providers.iter().find(|p| p["name"] == "ollama").unwrap();
    assert_eq!(ollama["api_key"], Value::Null);
    assert_eq!(ollama["api_key_set"], false);

    // ── 校验失败：default_provider 不在列表 → 400，且不落盘不改运行态 ──
    let (status, _) = put_settings(
        &app,
        json!({
            "default_provider": "nope",
            "providers": [{"name": "ollama", "base_url": "http://x/v1", "model": "m", "wire_api": "chat"}]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(state.agent().config().providers.len(), 3);

    // 校验失败：wire_api 非法 → 400
    let (status, _) = put_settings(
        &app,
        json!({
            "default_provider": "ollama",
            "providers": [{"name": "ollama", "base_url": "http://x/v1", "model": "m", "wire_api": "grpc"}]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // ── PUT：修改后保存并热生效 ──────────────────────────────────────
    // deepseek 不给 api_key 字段 → 保留原 env: 引用；随后再显式替换为明文。
    let (status, v) = put_settings(
        &app,
        json!({
            "default_provider": "ollama",
            "context_window": 64000,
            "max_tool_rounds": 10,
            "providers": [
                {"name": "ollama", "base_url": "http://localhost:11434/v1", "model": "qwen3:8b", "wire_api": "chat"},
                {"name": "deepseek", "base_url": "https://api.deepseek.com/v1", "model": "deepseek-reasoner", "wire_api": "chat"}
            ]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let providers = v["providers"].as_array().unwrap();
    assert_eq!(providers.len(), 2);
    let deepseek = providers.iter().find(|p| p["name"] == "deepseek").unwrap();
    assert_eq!(deepseek["api_key"], "env:DEEPSEEK_API_KEY"); // 保留成功
    assert_eq!(deepseek["model"], "deepseek-reasoner");

    // 运行态热替换立即可见
    let agent = state.agent();
    assert_eq!(agent.provider().name(), "ollama");
    assert_eq!(agent.provider().model(), "qwen3:8b");
    assert_eq!(agent.config().default_provider, "ollama");
    assert_eq!(agent.config().providers.len(), 2);

    // 磁盘落盘（重新读取验证）
    let saved = Config::load().unwrap();
    assert_eq!(saved.default_provider, "ollama");
    assert_eq!(saved.context_window, 64000);
    assert_eq!(saved.max_tool_rounds, 10);
    assert_eq!(saved.providers.len(), 2);

    // ── 明文密钥：PUT 后不回显（api_key=null + api_key_set=true）────
    let (status, v) = put_settings(
        &app,
        json!({
            "default_provider": "ollama",
            "providers": [
                {"name": "ollama", "base_url": "http://localhost:11434/v1", "model": "qwen3:8b", "wire_api": "chat"},
                {"name": "deepseek", "base_url": "https://api.deepseek.com/v1", "model": "deepseek-reasoner", "wire_api": "chat", "api_key": "sk-secret-123"}
            ]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let deepseek = v["providers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "deepseek")
        .unwrap()
        .clone();
    assert_eq!(deepseek["api_key"], Value::Null); // 明文不回显
    assert_eq!(deepseek["api_key_set"], true);

    // 磁盘上是明文；再次 PUT 不带 api_key 字段时保留它
    let saved = Config::load().unwrap();
    let deepseek = saved.providers.iter().find(|p| p.name == "deepseek").unwrap();
    assert_eq!(deepseek.api_key.as_deref(), Some("sk-secret-123"));

    let (status, v) = put_settings(
        &app,
        json!({
            "default_provider": "ollama",
            "providers": [
                {"name": "ollama", "base_url": "http://localhost:11434/v1", "model": "qwen3:8b", "wire_api": "chat"},
                {"name": "deepseek", "base_url": "https://api.deepseek.com/v1", "model": "deepseek-reasoner", "wire_api": "chat"}
            ]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let saved = Config::load().unwrap();
    let deepseek = saved.providers.iter().find(|p| p.name == "deepseek").unwrap();
    assert_eq!(deepseek.api_key.as_deref(), Some("sk-secret-123")); // 保留成功
    let _ = v;
}
