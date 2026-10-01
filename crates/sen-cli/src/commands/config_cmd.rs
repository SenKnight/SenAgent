//! `sen config`：显示 / 初始化 / 定位配置。

use anyhow::{Context, Result};
use sen_core::{paths, Config};

use crate::ConfigCmd;

pub fn run(cmd: Option<ConfigCmd>) -> Result<()> {
    match cmd.unwrap_or(ConfigCmd::Show) {
        ConfigCmd::Path => {
            println!("{}", paths::config_path().display());
        }
        ConfigCmd::Show => {
            let path = paths::config_path();
            if !path.exists() {
                Config::template().save().context("写入模板配置失败")?;
                println!("已生成默认配置: {}", path.display());
            }
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("读取 {} 失败", path.display()))?;
            println!("# {}\n", path.display());
            print!("{text}");
        }
        ConfigCmd::Init => {
            let path = paths::config_path();
            Config::template().save().context("写入模板配置失败")?;
            println!("已写入模板配置: {}", path.display());
        }
    }
    Ok(())
}
