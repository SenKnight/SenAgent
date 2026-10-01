//! 技能工具：load_skill（按需加载技能全文）/ create_skill（创建新技能）。

use std::sync::Arc;

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

use crate::error::{Error, Result};
use crate::skills::SkillManager;
use crate::tools::{schema_of, Tool};

pub struct LoadSkillTool {
    skills: Arc<SkillManager>,
}

impl LoadSkillTool {
    pub fn new(skills: Arc<SkillManager>) -> Self {
        Self { skills }
    }
}

#[derive(Deserialize, JsonSchema)]
struct LoadArgs {
    /// 技能名称（frontmatter 的 name，或相对目录路径如 `coding/git-helper`）
    name: String,
}

#[async_trait]
impl Tool for LoadSkillTool {
    fn name(&self) -> &'static str {
        "load_skill"
    }

    fn description(&self) -> &'static str {
        "加载指定技能的完整说明（SKILL.md 正文）。当任务与系统提示词中列出的技能描述匹配时，先加载技能，再按其中的步骤执行。"
    }

    fn parameters_schema(&self) -> Value {
        schema_of::<LoadArgs>()
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let args: LoadArgs =
            serde_json::from_value(args).map_err(|e| Error::Tool(format!("参数不合法: {e}")))?;
        let loaded = self
            .skills
            .load(&args.name)
            .map_err(|e| Error::Tool(e.to_string()))?;

        let mut out = format!("# 技能: {}\n", loaded.info.name);
        if !loaded.info.group.is_empty() {
            out.push_str(&format!("分组: {}\n", loaded.info.group));
        }
        out.push_str(&format!("路径: {}\n\n", loaded.info.path.display()));
        out.push_str(loaded.body.trim());
        out.push('\n');
        if !loaded.resources.is_empty() {
            out.push_str("\n## 附带资源（可用 read_file 读取，路径相对技能目录）\n");
            for r in &loaded.resources {
                out.push_str(&format!("- {}\n", r));
            }
        }
        Ok(out)
    }
}

pub struct CreateSkillTool {
    skills: Arc<SkillManager>,
}

impl CreateSkillTool {
    pub fn new(skills: Arc<SkillManager>) -> Self {
        Self { skills }
    }
}

#[derive(Deserialize, JsonSchema)]
struct CreateArgs {
    /// 技能目录：相对 ~/.agents/skills 的相对路径，支持多级分组（如 `coding/git-helper`）
    path: String,
    /// 技能名称（默认取目录最后一段）
    #[serde(default)]
    name: Option<String>,
    /// 一句话描述：说明该技能在什么场景下使用（会展示给模型做触发判断）
    #[serde(default)]
    description: Option<String>,
    /// SKILL.md 正文（Markdown）；若以 `---` 开头则视为包含 frontmatter 的完整文件内容
    #[serde(default)]
    content: Option<String>,
}

#[async_trait]
impl Tool for CreateSkillTool {
    fn name(&self) -> &'static str {
        "create_skill"
    }

    fn description(&self) -> &'static str {
        "创建一个新技能：在 ~/.agents/skills 下写入 SKILL.md（含 name/description frontmatter）。技能索引在下一轮对话自动生效。"
    }

    fn parameters_schema(&self) -> Value {
        schema_of::<CreateArgs>()
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let args: CreateArgs =
            serde_json::from_value(args).map_err(|e| Error::Tool(format!("参数不合法: {e}")))?;
        let file = self
            .skills
            .create(
                &args.path,
                args.name.as_deref(),
                args.description.as_deref(),
                args.content.as_deref(),
            )
            .map_err(|e| Error::Tool(e.to_string()))?;
        Ok(format!("已创建技能，写入 {}", file.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_then_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let skills = Arc::new(SkillManager::new(dir.path().join("skills")));

        let out = CreateSkillTool::new(skills.clone())
            .execute(serde_json::json!({
                "path": "coding/git-helper",
                "name": "git-helper",
                "description": "处理 git 常用操作",
                "content": "# git helper\n\n先运行 `git status`。\n",
            }))
            .await
            .unwrap();
        assert!(out.contains("已创建技能"));

        let out = LoadSkillTool::new(skills)
            .execute(serde_json::json!({ "name": "git-helper" }))
            .await
            .unwrap();
        assert!(out.contains("# 技能: git-helper"));
        assert!(out.contains("git status"));
    }

    #[tokio::test]
    async fn load_missing_skill_fails_with_hint() {
        let dir = tempfile::tempdir().unwrap();
        let skills = Arc::new(SkillManager::new(dir.path().join("skills")));
        let err = LoadSkillTool::new(skills)
            .execute(serde_json::json!({ "name": "nope" }))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("不存在"));
    }
}
