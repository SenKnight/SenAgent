//! `/api/files/*` 集成测试：只读文件树、文件预览、目录穿越与二进制防护。
//!
//! 单一测试函数内完成全部场景（设置进程级环境变量 SEN_AGENT_HOME /
//! SEN_AGENTS_DIR 指向临时目录，避免并行测试互相干扰与污染真实数据目录）。

use std::fs;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

use sen_core::{Agent, Config};
use sen_server::{build_router, AppState};

async fn body_json(resp: axum::response::Response) -> (StatusCode, Value) {
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, v)
}

async fn get(app: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    body_json(resp).await
}

#[tokio::test]
async fn files_tree_content_and_guard() {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("SEN_AGENT_HOME", dir.path().join("home"));
    std::env::set_var("SEN_AGENTS_DIR", dir.path().join("agents"));

    // 造产物目录与文件：文本（可预览）+ 二进制（不可预览）
    let artifacts = dir.path().join(".sen-agent").join("artifacts");
    fs::create_dir_all(&artifacts).unwrap();
    fs::write(artifacts.join("x.md"), "# 计划\n内容").unwrap();
    fs::write(artifacts.join("bin.dat"), [0xff, 0xfe, 0x00, 0x01]).unwrap();

    let config = Config::template();
    config.save().unwrap();
    let agent = Agent::from_config(config, Some("ollama"), dir.path().to_path_buf()).unwrap();
    let app = build_router(AppState::new(agent), None);

    // 根目录：包含隐藏项 .sen-agent（目录排在前面）
    let (status, v) = get(&app, "/api/files/tree").await;
    assert_eq!(status, StatusCode::OK);
    let arr = v.as_array().unwrap();
    let sen = arr.iter().find(|n| n["name"] == ".sen-agent").unwrap();
    assert_eq!(sen["is_dir"], true);
    assert_eq!(arr[0]["name"], ".sen-agent");

    // .sen-agent 下有 artifacts 目录
    let (status, v) = get(&app, "/api/files/tree?path=.sen-agent").await;
    assert_eq!(status, StatusCode::OK);
    assert!(v
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["name"] == "artifacts" && n["is_dir"] == true));

    // 文件内容预览
    let (status, v) = get(&app, "/api/files/content?path=.sen-agent/artifacts/x.md").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v["path"], ".sen-agent/artifacts/x.md");
    assert!(v["content"].as_str().unwrap().contains("计划"));

    // 目录穿越被拒绝（400）
    let (status, _) = get(&app, "/api/files/content?path=..").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // 二进制文件不支持预览（400）
    let (status, _) = get(&app, "/api/files/content?path=.sen-agent/artifacts/bin.dat").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
