//! `/api/sessions/{id}/plans` 集成测试：计划持久化后可通过 REST 读取。

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

use sen_core::{Agent, Config};
use sen_server::{build_router, AppState};

#[tokio::test]
async fn list_plans_returns_persisted() {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("SEN_AGENT_HOME", dir.path().join("home"));
    std::env::set_var("SEN_AGENTS_DIR", dir.path().join("agents"));

    let config = Config::template();
    config.save().unwrap();
    let agent = Agent::from_config(config, Some("ollama"), dir.path().to_path_buf()).unwrap();
    let app = build_router(AppState::new(agent.clone()), None);

    // 造会话与计划
    let session = agent.store().create_session("", None).unwrap();
    agent
        .store()
        .add_plan(&session.id, "## 计划\n1. 第一步")
        .unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/sessions/{}/plans", session.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(status, StatusCode::OK);
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["session_id"], session.id);
    assert_eq!(arr[0]["status"], "draft");
    assert!(arr[0]["content"].as_str().unwrap().contains("计划"));
}
