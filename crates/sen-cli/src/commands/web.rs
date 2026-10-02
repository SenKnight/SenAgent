//! `sen web`：以单文件方式启动 Web UI（前端资源已内嵌于二进制）。

use std::sync::Arc;

use anyhow::Result;

use super::build_agent;
use crate::web_assets::{is_built, EmbeddedWeb};

pub async fn run(provider: Option<String>, host: String, port: u16, open: bool) -> Result<()> {
    let agent = build_agent(provider.as_deref())?;
    if !is_built() {
        println!(
            "提示: 当前二进制内嵌的是占位页（前端未构建）；完整 Web UI 请先 `cd frontend && npm run build` 后重新安装 CLI"
        );
    }
    let url = format!("http://{host}:{port}");
    println!("SenAgent Web UI: {url}");
    if open {
        open_browser(&url);
    }
    let cfg = sen_server::ServerConfig {
        host,
        port,
        static_dir: None,
        assets: Some(Arc::new(EmbeddedWeb)),
    };
    sen_server::serve(agent, cfg).await
}

/// 尽力打开系统默认浏览器（失败仅忽略，不影响服务）。
fn open_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let cmd = ("open", vec![url]);
    #[cfg(target_os = "windows")]
    let cmd = ("cmd", vec!["/C", "start", "", url]);
    #[cfg(all(unix, not(target_os = "macos")))]
    let cmd = ("xdg-open", vec![url]);

    let _ = std::process::Command::new(cmd.0).args(cmd.1).spawn();
}
