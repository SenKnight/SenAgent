//! 系统提示词组装：基线人设 + 指导文件（AGENTS.md）+ 技能索引。

use std::path::{Path, PathBuf};

use crate::paths;

/// 一份被加载的指导文件。
#[derive(Debug, Clone)]
pub struct GuidanceFile {
    /// 作用域描述：`全局` 或 `项目 <目录>`
    pub scope: String,
    pub path: PathBuf,
    pub content: String,
}

/// 加载指导文件：全局 `~/.agents/AGENTS.md` + 从 CWD 向上逐级查找的项目级 AGENTS.md。
///
/// 返回顺序：全局在前；项目级由外到内（内层目录的内容最后出现，语义上优先级更高）。
pub fn load_guidance(cwd: &Path) -> Vec<GuidanceFile> {
    let global = paths::global_agents_md();
    let mut out = Vec::new();
    if let Ok(content) = std::fs::read_to_string(&global) {
        if !content.trim().is_empty() {
            out.push(GuidanceFile {
                scope: "全局".into(),
                path: global.clone(),
                content,
            });
        }
    }

    let cwd = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
    let mut chain: Vec<PathBuf> = Vec::new();
    let mut dir = Some(cwd.as_path());
    while let Some(d) = dir {
        chain.push(d.to_path_buf());
        dir = d.parent();
    }
    chain.reverse();

    for dir in chain {
        let file = dir.join("AGENTS.md");
        if file == global {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(&file) {
            if !content.trim().is_empty() {
                out.push(GuidanceFile {
                    scope: format!("项目 {}", dir.display()),
                    path: file,
                    content,
                });
            }
        }
    }
    out
}

/// 基线系统提示词（不含指导文件与技能索引）；`cwd` 为当前工作目录。
pub fn base_system_prompt(cwd: &Path) -> String {
    let cwd = cwd.display().to_string();
    format!(
        "你是 SenAgent，一个运行在用户本机上的个人 AI Agent。\n\n\
## 工作方式\n\
- 你是自主代理：用工具（shell、文件读写编辑、glob/grep 搜索、web 抓取等）直接完成任务，而不是只给建议。\n\
- 需要了解代码或文件时，先用 read_file / grep / glob 获取事实，不要凭猜测回答。\n\
- 修改文件前先读取确认原文；修改后如有可用的构建或测试命令，主动运行验证。\n\
- 工具报错时先分析原因再调整，不要盲目重复相同的调用。\n\
- 默认使用中文回复（除非用户使用其他语言）。\n\n\
## 产出物\n\
- 需要产出文档、方案、报告等交付物时，写入相对当前目录的 `.sen-agent/artifacts/`（父目录会自动创建），便于用户在界面右侧文件树查看。\n\n\
## 环境\n\
- 操作系统: {os}\n\
- 当前目录: {cwd}",
        os = std::env::consts::OS,
        cwd = cwd
    )
}

/// 组装最终系统提示词：基线 + 自定义附加 + 指导文件 + 技能索引。
pub fn assemble_system_prompt(
    extra: Option<&str>,
    guidance: &[GuidanceFile],
    skills_index: &str,
    cwd: &Path,
) -> String {
    let mut parts = vec![base_system_prompt(cwd)];
    if let Some(e) = extra {
        if !e.trim().is_empty() {
            parts.push(e.trim().to_string());
        }
    }
    if !guidance.is_empty() {
        let mut s =
            String::from("# 指导文件（AGENTS.md）\n\n以下内容来自用户的指导文件，必须优先遵守：\n");
        for g in guidance {
            s.push_str(&format!(
                "\n## 来源: {}（{}）\n\n{}\n",
                g.path.display(),
                g.scope,
                g.content.trim()
            ));
        }
        parts.push(s);
    }
    if !skills_index.trim().is_empty() {
        parts.push(skills_index.trim().to_string());
    }
    parts.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assemble_includes_all_parts() {
        let g = GuidanceFile {
            scope: "全局".into(),
            path: PathBuf::from("/tmp/AGENTS.md"),
            content: "必须使用中文回答".into(),
        };
        let out = assemble_system_prompt(Some("附加指令"), &[g], "## 可用技能\n\n- `x`: demo", Path::new("."));
        assert!(out.contains("SenAgent"));
        assert!(out.contains("附加指令"));
        assert!(out.contains("必须使用中文回答"));
        assert!(out.contains("可用技能"));
    }

    #[test]
    fn assemble_without_optional_parts() {
        let out = assemble_system_prompt(None, &[], "", Path::new("."));
        assert!(out.contains("SenAgent"));
        assert!(!out.contains("指导文件"));
    }

    #[test]
    fn load_guidance_reads_project_chain_outer_first() {
        let dir = tempfile::tempdir().unwrap();
        // 隔离全局文件（指向不存在的目录）
        std::env::set_var("SEN_AGENTS_DIR", dir.path().join("agents-assets"));
        std::fs::write(dir.path().join("AGENTS.md"), "outer rules").unwrap();
        std::fs::create_dir_all(dir.path().join("sub/inner")).unwrap();
        std::fs::write(dir.path().join("sub/AGENTS.md"), "inner rules").unwrap();

        let g = load_guidance(&dir.path().join("sub/inner"));
        let contents: Vec<&str> = g.iter().map(|f| f.content.as_str()).collect();
        let outer = contents.iter().position(|c| *c == "outer rules").unwrap();
        let inner = contents.iter().position(|c| *c == "inner rules").unwrap();
        assert!(outer < inner, "外层目录的指导文件应排在内层之前");

        std::env::remove_var("SEN_AGENTS_DIR");
    }
}
