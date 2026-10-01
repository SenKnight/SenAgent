//! SenAgent 桌面端：Tauri 2 壳，进程内嵌 sen-core + sen-server（单进程单二进制，无 sidecar）。
//!
//! 启动流程：
//! 1. 绑定 `127.0.0.1` 随机端口（避免端口冲突）；
//! 2. 在应用进程内启动 axum 服务（WebSocket 聊天 + REST 会话 + 托管前端静态资源）；
//! 3. 打开窗口指向本地服务 —— 与浏览器访问行为完全一致。
//!
//! 前端资源定位：优先使用打包资源（bundle resources 中的 `frontend-dist`），
//! 开发时回退到仓库的 `frontend/dist`（需先 `npm run build`）。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

use sen_core::{Agent, Config};

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                match start_backend(&handle).await {
                    Ok(port) => {
                        let url = format!("http://127.0.0.1:{port}");
                        let win_handle = handle.clone();
                        let _ = handle.run_on_main_thread(move || {
                            let parsed = match url.parse() {
                                Ok(u) => u,
                                Err(e) => {
                                    eprintln!("本地服务 URL 非法: {e}");
                                    return;
                                }
                            };
                            if let Err(e) = WebviewWindowBuilder::new(
                                &win_handle,
                                "main",
                                WebviewUrl::External(parsed),
                            )
                            .title("SenAgent")
                            .inner_size(1200.0, 820.0)
                            .min_inner_size(860.0, 600.0)
                            .build()
                            {
                                eprintln!("创建窗口失败: {e}");
                            }
                        });
                    }
                    Err(e) => eprintln!("后端启动失败: {e}"),
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Tauri 应用运行失败");
}

/// 启动内嵌后端（axum），返回实际监听端口。
async fn start_backend(app: &tauri::AppHandle) -> anyhow::Result<u16> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let port = listener.local_addr()?.port();

    let config = Config::load()?;
    let cwd = std::env::current_dir()?;
    let agent = Agent::from_config(config, None, cwd)?;
    let static_dir = find_static_dir(app);

    tokio::spawn(async move {
        if let Err(e) = sen_server::serve_on(listener, agent, static_dir).await {
            eprintln!("后端服务退出: {e}");
        }
    });
    Ok(port)
}

/// 定位前端构建产物：优先打包资源目录，其次开发目录。
fn find_static_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    if let Ok(res) = app.path().resource_dir() {
        let packaged = res.join("frontend-dist");
        if packaged.is_dir() {
            return Some(packaged);
        }
    }
    let dev = PathBuf::from("frontend/dist");
    dev.is_dir().then_some(dev)
}
