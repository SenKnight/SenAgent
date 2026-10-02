//! 目录约定。
//!
//! - `~/.sen-agent/`：SenAgent 自身的数据目录（config.toml、data.db、logs、history）
//! - `~/.agents/`：跨 agent 工具共享的用户级资产目录（skills/、AGENTS.md）
//! - `<项目工作目录>/.sen-agent/`：项目级私有目录（当前为产出物 `artifacts/`）
//!
//! 三者职责分离：应用私有数据、可共享的用户级资产、随项目走的项目级产出。
//! 全局目录均支持环境变量覆盖，便于测试与自定义部署。

use std::path::{Path, PathBuf};

/// SenAgent 自身数据目录：默认 `~/.sen-agent`，可用 `SEN_AGENT_HOME` 覆盖。
pub fn base_dir() -> PathBuf {
    if let Some(v) = std::env::var_os("SEN_AGENT_HOME") {
        return PathBuf::from(v);
    }
    home().join(".sen-agent")
}

pub fn config_path() -> PathBuf {
    base_dir().join("config.toml")
}

pub fn db_path() -> PathBuf {
    base_dir().join("data.db")
}

pub fn logs_dir() -> PathBuf {
    base_dir().join("logs")
}

pub fn history_path() -> PathBuf {
    base_dir().join("history.txt")
}

/// 用户级 agent 资产目录：默认 `~/.agents`，可用 `SEN_AGENTS_DIR` 覆盖。
pub fn agents_dir() -> PathBuf {
    if let Some(v) = std::env::var_os("SEN_AGENTS_DIR") {
        return PathBuf::from(v);
    }
    home().join(".agents")
}

/// 技能目录：默认 `~/.agents/skills`（支持任意层级分组子目录）。
pub fn skills_dir() -> PathBuf {
    agents_dir().join("skills")
}

/// 全局指导文件：默认 `~/.agents/AGENTS.md`。
pub fn global_agents_md() -> PathBuf {
    agents_dir().join("AGENTS.md")
}

/// 项目级私有目录：工作目录下的 `.sen-agent`（与全局 `~/.sen-agent` 语义不同）。
pub fn sen_dir(cwd: &Path) -> PathBuf {
    cwd.join(".sen-agent")
}

/// 项目产出物目录：`<cwd>/.sen-agent/artifacts`。
///
/// Agent 生成的文档/方案/报告等交付物默认写入此处，供界面右侧文件树查看。
pub fn artifacts_dir(cwd: &Path) -> PathBuf {
    sen_dir(cwd).join("artifacts")
}

pub fn ensure_base_dir() -> std::io::Result<()> {
    std::fs::create_dir_all(base_dir())
}

/// 用户主目录（目录浏览器默认起点）。
pub fn home_dir() -> PathBuf {
    home()
}

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}
