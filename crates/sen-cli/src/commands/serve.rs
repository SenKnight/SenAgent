//! `sen serve`：启动 Web 服务（浏览器 / 桌面端共用入口）。

use std::path::PathBuf;

use anyhow::Result;

use super::build_agent;

pub async fn run(
    provider: Option<String>,
    host: String,
    port: u16,
    static_dir: Option<PathBuf>,
) -> Result<()> {
    let agent = build_agent(provider.as_deref())?;
    // 未显式指定时自动探测前端构建产物
    let static_dir = static_dir.or_else(|| {
        let p = PathBuf::from("frontend/dist");
        p.is_dir().then_some(p)
    });
    if let Some(dir) = &static_dir {
        println!("托管前端静态资源: {}", dir.display());
    } else {
        println!("提示: 未找到前端产物 frontend/dist（仅提供 API），可在项目根运行或用 --static-dir <路径> 指定");
    }
    let cfg = sen_server::ServerConfig {
        host: host.clone(),
        port,
        static_dir,
    };
    println!("SenAgent Web 服务: http://{host}:{port}");
    sen_server::serve(agent, cfg).await
}
