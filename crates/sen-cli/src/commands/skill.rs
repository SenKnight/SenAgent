//! `sen skill`：技能列出 / 查看 / 创建（~/.agents/skills，支持分层分组）。

use anyhow::{Context, Result};
use sen_core::{paths, SkillManager};

use crate::SkillCmd;

pub async fn run(cmd: SkillCmd) -> Result<()> {
    let manager = SkillManager::new(paths::skills_dir());
    match cmd {
        SkillCmd::List => {
            let skills = manager.scan();
            if skills.is_empty() {
                println!("暂无技能。目录: {}", manager.root().display());
                println!(
                    "创建示例: sen skill new coding/git-helper --description \"处理 git 常见操作\""
                );
                return Ok(());
            }
            println!("技能目录: {}\n", manager.root().display());
            let mut current_group = "\u{0}".to_string();
            for s in &skills {
                if s.group != current_group {
                    current_group = s.group.clone();
                    println!(
                        "[{}]",
                        if current_group.is_empty() {
                            "顶层"
                        } else {
                            &current_group
                        }
                    );
                }
                println!("  {:<24} {}", s.name, s.description);
            }
        }
        SkillCmd::Show { name } => {
            let loaded = manager
                .load(&name)
                .with_context(|| format!("加载技能 `{name}` 失败"))?;
            println!(
                "# {} （分组: {}）",
                loaded.info.name,
                if loaded.info.group.is_empty() {
                    "无"
                } else {
                    &loaded.info.group
                }
            );
            println!("路径: {}\n", loaded.info.path.display());
            println!("{}", loaded.body.trim());
            if !loaded.resources.is_empty() {
                println!("\n附带资源:");
                for r in loaded.resources {
                    println!("  - {r}");
                }
            }
        }
        SkillCmd::New {
            path,
            name,
            description,
            content_file,
        } => {
            let content = match content_file {
                Some(f) => Some(
                    std::fs::read_to_string(&f)
                        .with_context(|| format!("读取 {} 失败", f.display()))?,
                ),
                None => None,
            };
            let file = manager.create(
                &path,
                name.as_deref(),
                description.as_deref(),
                content.as_deref(),
            )?;
            println!("已创建技能: {}", file.display());
            println!("提示: 技能索引会在下一轮对话自动生效");
        }
        SkillCmd::Dir => {
            println!("{}", manager.root().display());
        }
    }
    Ok(())
}
