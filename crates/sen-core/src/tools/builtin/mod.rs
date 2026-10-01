//! 内置工具集合。

pub mod files;
pub mod search;
pub mod shell;
pub mod skill_tools;
pub mod web;

use std::sync::Arc;

use crate::skills::SkillManager;
use crate::tools::ToolRegistry;

/// 注册全部内置工具。
///
/// 技能相关工具（load_skill / create_skill）需要共享 [`SkillManager`]。
pub fn register_builtin_tools(registry: &mut ToolRegistry, skills: Arc<SkillManager>) {
    registry.register(shell::ShellTool);
    registry.register(files::ReadFileTool);
    registry.register(files::WriteFileTool);
    registry.register(files::EditFileTool);
    registry.register(search::GlobTool);
    registry.register(search::GrepTool);
    registry.register(web::FetchTool);
    registry.register(skill_tools::LoadSkillTool::new(skills.clone()));
    registry.register(skill_tools::CreateSkillTool::new(skills));
}
