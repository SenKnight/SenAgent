//! 技能系统：`~/.agents/skills` 分层扫描、解析（SKILL.md + frontmatter）与创建。
//!
//! 目录支持任意层级分组，例如：
//!
//! ```text
//! ~/.agents/skills/
//! ├── coding/
//! │   └── git-helper/
//! │       └── SKILL.md
//! └── writing/
//!     └── blog/
//!         └── SKILL.md
//! ```
//!
//! SKILL.md 以 YAML frontmatter 声明 `name` / `description`；元数据注入系统
//! 提示词做索引，正文由 `load_skill` 工具按需加载（渐进式披露）。
//! 扫描不做缓存（按需实时扫描），因此目录变化天然热生效。

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use crate::error::{Error, Result};

/// 技能元数据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skill {
    pub name: String,
    pub description: String,
    /// 相对技能根目录的分组路径（如 `coding`；顶层技能为空串）
    pub group: String,
    /// SKILL.md 的绝对路径
    pub path: PathBuf,
}

/// 加载后的技能（正文 + 附带资源清单）。
#[derive(Debug, Clone)]
pub struct LoadedSkill {
    pub info: Skill,
    /// frontmatter 之后的正文
    pub body: String,
    /// 技能目录下的其他文件（相对技能目录的路径）
    pub resources: Vec<String>,
}

/// 技能管理器：负责扫描、加载与创建。
pub struct SkillManager {
    root: PathBuf,
}

impl SkillManager {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 递归扫描技能目录，返回全部技能元数据（按分组 + 名称排序）。
    pub fn scan(&self) -> Vec<Skill> {
        let mut skills = Vec::new();
        if !self.root.is_dir() {
            return skills;
        }
        for entry in walkdir::WalkDir::new(&self.root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !is_hidden(e))
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() || entry.file_name() != "SKILL.md" {
                continue;
            }
            if let Some(skill) = self.skill_from_path(entry.path()) {
                skills.push(skill);
            }
        }
        skills.sort_by(|a, b| (&a.group, &a.name).cmp(&(&b.group, &b.name)));
        skills
    }

    /// 按名称查找技能；名称既可以是 frontmatter 的 `name`，
    /// 也可以是相对目录路径（如 `coding/git-helper`）。
    pub fn find(&self, name: &str) -> Option<Skill> {
        let name = name.trim().trim_end_matches('/');
        self.scan()
            .into_iter()
            .find(|s| s.name == name || self.rel_dir(s) == name)
    }

    /// 加载技能全文（正文 + 资源清单）。
    pub fn load(&self, name: &str) -> Result<LoadedSkill> {
        let info = self.find(name).ok_or_else(|| {
            let names: Vec<String> = self.scan().iter().map(|s| s.name.clone()).collect();
            if names.is_empty() {
                Error::Other(format!(
                    "技能 `{name}` 不存在：{} 下暂无任何技能",
                    self.root.display()
                ))
            } else {
                Error::Other(format!(
                    "技能 `{name}` 不存在；可用技能: {}",
                    names.join(", ")
                ))
            }
        })?;
        let content = std::fs::read_to_string(&info.path)
            .map_err(|e| Error::Other(format!("读取 {} 失败: {e}", info.path.display())))?;
        let (_, body) = parse_frontmatter(&content);
        let resources = list_resources(&info.path);
        Ok(LoadedSkill {
            info,
            body,
            resources,
        })
    }

    /// 创建技能：在 `root/<rel_path>/SKILL.md` 写入带 frontmatter 的技能文件。
    ///
    /// `content` 若以 `---` 开头则视为完整 SKILL.md 内容直接写入；
    /// 否则作为正文与自动生成的 frontmatter 组合。
    pub fn create(
        &self,
        rel_path: &str,
        name: Option<&str>,
        description: Option<&str>,
        content: Option<&str>,
    ) -> Result<PathBuf> {
        let rel = validate_rel_path(rel_path)?;
        let dir = self.root.join(&rel);
        let file = dir.join("SKILL.md");
        if file.exists() {
            return Err(Error::Other(format!(
                "技能已存在: {}（如需修改请直接编辑该文件）",
                file.display()
            )));
        }
        let default_name = rel
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "skill".to_string());
        let name = name
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(default_name.as_str())
            .to_string();
        let description = description
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("{name} 技能（待补充描述）"));

        std::fs::create_dir_all(&dir)
            .map_err(|e| Error::Other(format!("创建目录 {} 失败: {e}", dir.display())))?;

        let text = match content {
            Some(c) if c.trim_start().starts_with("---") => c.to_string(),
            other => {
                let body = other
                    .map(str::to_string)
                    .unwrap_or_else(|| default_body(&name, &description));
                format!("---\nname: {name}\ndescription: {description}\n---\n\n{body}")
            }
        };
        std::fs::write(&file, text)
            .map_err(|e| Error::Other(format!("写入 {} 失败: {e}", file.display())))?;
        Ok(file)
    }

    /// 生成注入系统提示词的技能索引（渐进式披露：只给名称与描述）。
    /// 没有任何技能时返回空串。
    pub fn index_prompt(&self) -> String {
        let skills = self.scan();
        if skills.is_empty() {
            return String::new();
        }
        let mut out = String::from(
            "## 可用技能\n\n当任务与某技能描述匹配时，先调用 `load_skill` 工具读取技能全文，再按其中的步骤执行。\n\n",
        );
        for s in &skills {
            let loc = if s.group.is_empty() {
                format!("`{}`", s.name)
            } else {
                format!("`{}`（{} 分组）", s.name, s.group)
            };
            out.push_str(&format!("- {loc}: {}\n", s.description));
        }
        out
    }

    fn skill_from_path(&self, path: &Path) -> Option<Skill> {
        let dir = path.parent()?;
        let rel = dir.strip_prefix(&self.root).ok()?;
        let group = rel
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let dir_name = dir.file_name()?.to_string_lossy().to_string();

        let content = std::fs::read_to_string(path).ok()?;
        let (meta, body) = parse_frontmatter(&content);
        let name = meta
            .get("name")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or(dir_name);
        let description = meta
            .get("description")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| first_paragraph(&body))
            .unwrap_or_default();
        Some(Skill {
            name,
            description,
            group,
            path: path.to_path_buf(),
        })
    }

    /// 技能目录相对根目录的路径（如 `coding/git-helper`）。
    fn rel_dir(&self, s: &Skill) -> String {
        s.path
            .parent()
            .and_then(|d| d.strip_prefix(&self.root).ok())
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default()
    }
}

/// 解析 SKILL.md 开头的简单 YAML frontmatter（`key: value` 单行键值对）。
///
/// 返回（元数据, 正文）。无 frontmatter 或未闭合时，元数据为空、正文为原文。
pub(crate) fn parse_frontmatter(content: &str) -> (BTreeMap<String, String>, String) {
    let mut meta = BTreeMap::new();
    let text = content.strip_prefix('\u{feff}').unwrap_or(content);
    let mut lines = text.lines();
    if lines.next().map(|l| l.trim()) != Some("---") {
        return (meta, text.to_string());
    }
    let mut body_lines: Vec<&str> = Vec::new();
    let mut closed = false;
    for line in lines {
        if !closed {
            if line.trim() == "---" {
                closed = true;
            } else if let Some((k, v)) = line.split_once(':') {
                let k = k.trim().to_string();
                let v = v
                    .trim()
                    .trim_matches(|c| c == '"' || c == '\'')
                    .trim()
                    .to_string();
                if !k.is_empty() && !k.starts_with('#') {
                    meta.insert(k, v);
                }
            }
        } else {
            body_lines.push(line);
        }
    }
    if !closed {
        return (BTreeMap::new(), text.to_string());
    }
    let body = body_lines.join("\n");
    (meta, body.trim_start_matches('\n').to_string())
}

fn first_paragraph(body: &str) -> Option<String> {
    for line in body.lines() {
        let t = line.trim();
        if !t.is_empty() && !t.starts_with('#') {
            return Some(crate::util::truncate(t, 200));
        }
    }
    None
}

fn default_body(name: &str, description: &str) -> String {
    format!(
        "# {name}\n\n{description}\n\n## 何时使用\n\n（描述触发本技能的场景）\n\n## 步骤\n\n1. （第一步）\n2. （第二步）\n\n## 注意事项\n\n- （补充说明）\n"
    )
}

/// 列出技能目录下的附带资源文件（相对技能目录路径，最多 50 个）。
fn list_resources(skill_path: &Path) -> Vec<String> {
    let Some(dir) = skill_path.parent() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(dir)
        .max_depth(4)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() || entry.file_name() == "SKILL.md" {
            continue;
        }
        if let Ok(rel) = entry.path().strip_prefix(dir) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
        if out.len() >= 50 {
            break;
        }
    }
    out.sort();
    out
}

/// 校验技能相对路径：非空、非绝对、不含 `..` 与隐藏目录。
fn validate_rel_path(rel_path: &str) -> Result<PathBuf> {
    let rel = rel_path.trim().trim_end_matches(['/', '\\']);
    if rel.is_empty() {
        return Err(Error::Other("技能目录不能为空".into()));
    }
    let path = Path::new(rel);
    if path.is_absolute() {
        return Err(Error::Other(
            "技能目录必须是相对 ~/.agents/skills 的相对路径".into(),
        ));
    }
    for comp in path.components() {
        match comp {
            Component::Normal(name) => {
                if name.to_string_lossy().starts_with('.') {
                    return Err(Error::Other("技能目录不能包含隐藏目录".into()));
                }
            }
            _ => {
                return Err(Error::Other(
                    "技能目录不能包含 `..` 等特殊路径成分".into(),
                ))
            }
        }
    }
    Ok(path.to_path_buf())
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry.depth() > 0
        && entry
            .file_name()
            .to_str()
            .map(|s| s.starts_with('.'))
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_frontmatter_extracts_fields() {
        let (meta, body) =
            parse_frontmatter("---\nname: git-helper\ndescription: \"Git 助手\"\n---\n\n# 标题\n正文");
        assert_eq!(meta.get("name").map(String::as_str), Some("git-helper"));
        assert_eq!(meta.get("description").map(String::as_str), Some("Git 助手"));
        assert!(body.starts_with("# 标题"));
    }

    #[test]
    fn parse_without_frontmatter() {
        let (meta, body) = parse_frontmatter("# Hello\ncontent");
        assert!(meta.is_empty());
        assert_eq!(body, "# Hello\ncontent");
    }

    #[test]
    fn create_scan_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = SkillManager::new(dir.path().join("skills"));
        let file = mgr
            .create("coding/git-helper", None, Some("处理 git 操作"), None)
            .unwrap();
        assert!(file.ends_with("coding/git-helper/SKILL.md"));

        let skills = mgr.scan();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "git-helper");
        assert_eq!(skills[0].group, "coding");
        assert_eq!(skills[0].description, "处理 git 操作");

        let loaded = mgr.load("git-helper").unwrap();
        assert!(loaded.body.contains("git-helper"));
        // 相对目录路径也可定位
        assert!(mgr.load("coding/git-helper").is_ok());
    }

    #[test]
    fn create_rejects_bad_paths() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = SkillManager::new(dir.path());
        assert!(mgr.create("../escape", None, None, None).is_err());
        assert!(mgr.create("/abs/path", None, None, None).is_err());
        assert!(mgr.create("", None, None, None).is_err());
        assert!(mgr.create(".hidden/x", None, None, None).is_err());
    }

    #[test]
    fn create_duplicate_fails() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = SkillManager::new(dir.path().join("skills"));
        mgr.create("demo", None, None, None).unwrap();
        assert!(mgr.create("demo", None, None, None).is_err());
    }

    #[test]
    fn index_prompt_lists_skills() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = SkillManager::new(dir.path().join("skills"));
        assert!(mgr.index_prompt().is_empty());
        mgr.create("git-helper", Some("git-helper"), Some("Git 操作"), None)
            .unwrap();
        let idx = mgr.index_prompt();
        assert!(idx.contains("git-helper"));
        assert!(idx.contains("Git 操作"));
        assert!(idx.contains("load_skill"));
    }
}
