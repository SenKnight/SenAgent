//! 搜索工具：glob 文件匹配 / grep 内容检索。

use std::path::{Path, PathBuf, MAIN_SEPARATOR};

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

use crate::error::{Error, Result};
use crate::tools::{schema_of, Tool};
use crate::util::truncate;

/// grep 遍历时跳过的目录名。
const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".venv",
    "venv",
    "__pycache__",
    ".next",
    ".idea",
    ".cache",
];

/// 单文件最大扫描体积（超过则跳过）。
const MAX_FILE_SIZE: u64 = 2 * 1024 * 1024;

pub struct GlobTool;

#[derive(Deserialize, JsonSchema)]
struct GlobArgs {
    /// glob 模式，如 `**/*.rs`（递归匹配需包含 `**`）
    pattern: String,
    /// 搜索根目录（可选，默认当前目录）
    #[serde(default)]
    path: Option<String>,
    /// 结果上限（默认 100，最大 1000）
    #[serde(default)]
    max_results: Option<usize>,
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &'static str {
        "glob"
    }

    fn description(&self) -> &'static str {
        "按 glob 模式查找文件路径，如 `**/*.rs`、`src/**/*.toml`。结果按路径排序。"
    }

    fn parameters_schema(&self) -> Value {
        schema_of::<GlobArgs>()
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let args: GlobArgs =
            serde_json::from_value(args).map_err(|e| Error::Tool(format!("参数不合法: {e}")))?;
        let base = args.path.unwrap_or_else(|| ".".into());
        let full = if Path::new(&args.pattern).is_absolute() {
            args.pattern.clone()
        } else {
            format!(
                "{}/{}",
                base.trim_end_matches(['/', '\\']),
                args.pattern
            )
        };
        let limit = args.max_results.unwrap_or(100).clamp(1, 1000);

        let mut matches: Vec<PathBuf> = Vec::new();
        let mut total = 0usize;
        for entry in
            glob::glob(&full).map_err(|e| Error::Tool(format!("glob 模式不合法: {e}")))?
        {
            match entry {
                Ok(p) => {
                    total += 1;
                    if matches.len() < limit {
                        matches.push(p);
                    }
                }
                Err(_) => continue,
            }
        }
        if matches.is_empty() {
            return Ok(format!("未找到匹配 `{}` 的文件", args.pattern));
        }
        matches.sort();

        let mut out = format!(
            "找到 {total} 个匹配 `{}`（显示 {} 个）:\n",
            args.pattern,
            matches.len()
        );
        for p in &matches {
            out.push_str(&display_path(p, &base));
            out.push('\n');
        }
        Ok(truncate(&out, 20_000))
    }
}

pub struct GrepTool;

#[derive(Deserialize, JsonSchema)]
struct GrepArgs {
    /// 正则表达式
    pattern: String,
    /// 搜索根目录（可选，默认当前目录）
    #[serde(default)]
    path: Option<String>,
    /// 文件名过滤 glob（可选，如 `*.rs`）
    #[serde(default)]
    glob: Option<String>,
    /// 忽略大小写（默认 false）
    #[serde(default)]
    case_insensitive: Option<bool>,
    /// 匹配行数上限（默认 100，最大 500）
    #[serde(default)]
    max_results: Option<usize>,
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &'static str {
        "grep"
    }

    fn description(&self) -> &'static str {
        "在文件中按正则检索内容，返回 `路径:行号: 内容`。默认跳过 .git/node_modules/target 等目录与超过 2MB 的文件。"
    }

    fn parameters_schema(&self) -> Value {
        schema_of::<GrepArgs>()
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let args: GrepArgs =
            serde_json::from_value(args).map_err(|e| Error::Tool(format!("参数不合法: {e}")))?;
        let re = regex::RegexBuilder::new(&args.pattern)
            .case_insensitive(args.case_insensitive.unwrap_or(false))
            .build()
            .map_err(|e| Error::Tool(format!("正则表达式不合法: {e}")))?;
        let base = PathBuf::from(args.path.unwrap_or_else(|| ".".into()));
        let glob_pat = args
            .glob
            .as_deref()
            .map(glob::Pattern::new)
            .transpose()
            .map_err(|e| Error::Tool(format!("glob 过滤不合法: {e}")))?;
        let limit = args.max_results.unwrap_or(100).clamp(1, 500);

        let mut hits: Vec<String> = Vec::new();
        let mut files_scanned = 0usize;
        let mut files_matched = 0usize;
        let mut truncated = false;

        'outer: for entry in walkdir::WalkDir::new(&base)
            .into_iter()
            .filter_entry(|e| !is_skipped(e))
        {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            if !entry.file_type().is_file() {
                continue;
            }
            if let Some(pat) = &glob_pat {
                if !pat.matches(&entry.file_name().to_string_lossy()) {
                    continue;
                }
            }
            if entry.metadata().map(|m| m.len()).unwrap_or(0) > MAX_FILE_SIZE {
                continue;
            }
            let Ok(bytes) = std::fs::read(entry.path()) else {
                continue;
            };
            let Ok(content) = String::from_utf8(bytes) else {
                continue;
            };
            files_scanned += 1;
            let mut file_had = false;
            for (i, line) in content.lines().enumerate() {
                if !re.is_match(line) {
                    continue;
                }
                file_had = true;
                if hits.len() >= limit {
                    truncated = true;
                    break 'outer;
                }
                hits.push(format!(
                    "{}:{}: {}",
                    normalize_path(entry.path()),
                    i + 1,
                    clip(line.trim_end(), 300)
                ));
            }
            if file_had {
                files_matched += 1;
            }
        }

        if hits.is_empty() {
            return Ok(format!(
                "未找到匹配 `{}` 的内容（扫描 {} 个文件）",
                args.pattern, files_scanned
            ));
        }
        let mut out = format!(
            "匹配 {} 行（{} 个文件命中，共扫描 {} 个文件）:\n",
            hits.len(),
            files_matched,
            files_scanned
        );
        for h in &hits {
            out.push_str(h);
            out.push('\n');
        }
        if truncated {
            out.push_str(&format!("（已达上限 {limit}，可能还有更多结果）\n"));
        }
        Ok(truncate(&out, 20_000))
    }
}

/// 展示路径：优先相对搜索根目录，并统一 `/` 分隔符。
fn display_path(p: &Path, base: &str) -> String {
    let rel = p.strip_prefix(Path::new(base)).unwrap_or(p);
    normalize_path(rel)
}

/// 输出路径统一使用 `/` 分隔符（Windows 原生为 `\`，统一后跨平台一致且与工具输入习惯一致）。
fn normalize_path(p: &Path) -> String {
    p.display().to_string().replace(MAIN_SEPARATOR, "/")
}

/// 截断单行（不追加统计说明，仅省略号）。
fn clip(line: &str, max: usize) -> String {
    if line.chars().count() <= max {
        return line.to_string();
    }
    let mut s: String = line.chars().take(max).collect();
    s.push('…');
    s
}

fn is_skipped(entry: &walkdir::DirEntry) -> bool {
    if entry.depth() == 0 {
        return false;
    }
    if !entry.file_type().is_dir() {
        return false;
    }
    let name = entry.file_name().to_string_lossy().to_string();
    name.starts_with('.') || SKIP_DIRS.contains(&name.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn glob_finds_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "fn main() {}").unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/b.rs"), "fn other() {}").unwrap();
        let out = GlobTool
            .execute(serde_json::json!({
                "pattern": "**/*.rs",
                "path": dir.path().to_string_lossy(),
            }))
            .await
            .unwrap();
        assert!(out.contains("a.rs"));
        assert!(out.contains("sub/b.rs"));
    }

    #[tokio::test]
    async fn grep_finds_lines_with_line_numbers() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("x.txt"), "hello world\nfoo bar\n").unwrap();
        let out = GrepTool
            .execute(serde_json::json!({
                "pattern": "foo\\s+bar",
                "path": dir.path().to_string_lossy(),
            }))
            .await
            .unwrap();
        assert!(out.contains("x.txt:2: foo bar"));
    }

    #[tokio::test]
    async fn grep_skips_target_dirs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("target")).unwrap();
        std::fs::write(dir.path().join("target/hit.txt"), "needle\n").unwrap();
        std::fs::write(dir.path().join("hit.txt"), "needle\n").unwrap();
        let out = GrepTool
            .execute(serde_json::json!({
                "pattern": "needle",
                "path": dir.path().to_string_lossy(),
            }))
            .await
            .unwrap();
        assert!(out.contains("hit.txt"));
        assert!(!out.contains("target/hit.txt"));
    }
}
