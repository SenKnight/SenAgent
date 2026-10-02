//! 文件工具：read_file / write_file / edit_file。

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;

use crate::error::{Error, Result};
use crate::tools::{resolve_path, schema_of, Tool};
use crate::util::truncate;

pub struct ReadFileTool;

#[derive(Deserialize, JsonSchema)]
struct ReadArgs {
    /// 文件路径
    path: String,
    /// 起始行号（1-based，可选）
    #[serde(default)]
    offset: Option<u64>,
    /// 最多读取行数（可选）
    #[serde(default)]
    limit: Option<u64>,
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &'static str {
        "read_file"
    }

    fn description(&self) -> &'static str {
        "读取文本文件内容，可指定起始行（offset）与行数上限（limit）。"
    }

    fn parameters_schema(&self) -> Value {
        schema_of::<ReadArgs>()
    }

    async fn execute(&self, args: Value, base: &Path) -> Result<String> {
        let args: ReadArgs = serde_json::from_value(args).map_err(|e| Error::Tool(format!("参数不合法: {e}")))?;
        let path = resolve_path(base, &args.path);
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| Error::Tool(format!("读取 {} 失败: {e}（二进制文件请用 shell 工具处理）", args.path)))?;

        let lines: Vec<&str> = content.lines().collect();
        let start = args.offset.unwrap_or(1).max(1) as usize - 1;
        if start >= lines.len() && !lines.is_empty() {
            return Ok(format!(
                "(文件共 {} 行，起始行 {} 超出范围)",
                lines.len(),
                start + 1
            ));
        }
        if lines.is_empty() {
            return Ok("(空文件)".into());
        }
        let end = args
            .limit
            .map(|l| start.saturating_add(l as usize))
            .unwrap_or(lines.len())
            .min(lines.len());
        let slice = lines[start..end].join("\n");
        Ok(format!(
            "[共 {} 行，显示 {}-{}]\n{}",
            lines.len(),
            start + 1,
            end,
            truncate(&slice, 100_000)
        ))
    }
}

pub struct WriteFileTool;

#[derive(Deserialize, JsonSchema)]
struct WriteArgs {
    /// 文件路径（父目录不存在时自动创建）
    path: String,
    /// 要写入的完整内容
    content: String,
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &'static str {
        "write_file"
    }

    fn description(&self) -> &'static str {
        "写入文件（覆盖已有内容）；父目录不存在时自动创建。"
    }

    fn parameters_schema(&self) -> Value {
        schema_of::<WriteArgs>()
    }

    async fn execute(&self, args: Value, base: &Path) -> Result<String> {
        let args: WriteArgs = serde_json::from_value(args).map_err(|e| Error::Tool(format!("参数不合法: {e}")))?;
        let path = resolve_path(base, &args.path);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| Error::Tool(format!("创建目录 {} 失败: {e}", parent.display())))?;
            }
        }
        tokio::fs::write(&path, &args.content)
            .await
            .map_err(|e| Error::Tool(format!("写入 {} 失败: {e}", args.path)))?;
        Ok(format!(
            "已写入 {} 字节到 {}",
            args.content.len(),
            args.path
        ))
    }
}

pub struct EditFileTool;

#[derive(Deserialize, JsonSchema)]
struct EditArgs {
    /// 文件路径
    path: String,
    /// 要被替换的原文（必须精确匹配，含缩进与换行）
    old_string: String,
    /// 替换后的新内容
    new_string: String,
    /// 是否替换全部匹配（默认 false，要求唯一匹配）
    #[serde(default)]
    replace_all: Option<bool>,
}

#[async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &'static str {
        "edit_file"
    }

    fn description(&self) -> &'static str {
        "在文件中做精确字符串替换（先读文件确认原文）。默认要求 old_string 唯一匹配；replace_all 可替换全部。"
    }

    fn parameters_schema(&self) -> Value {
        schema_of::<EditArgs>()
    }

    async fn execute(&self, args: Value, base: &Path) -> Result<String> {
        let args: EditArgs = serde_json::from_value(args).map_err(|e| Error::Tool(format!("参数不合法: {e}")))?;
        let path = resolve_path(base, &args.path);
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| Error::Tool(format!("读取 {} 失败: {e}", args.path)))?;

        let count = content.matches(&args.old_string).count();
        if count == 0 {
            return Err(Error::Tool(format!(
                "在 {} 中未找到要替换的内容；请先 read_file 确认原文（注意空白字符与换行需完全一致）",
                args.path
            )));
        }
        let replace_all = args.replace_all.unwrap_or(false);
        if count > 1 && !replace_all {
            return Err(Error::Tool(format!(
                "old_string 在 {} 中匹配到 {count} 处，不唯一；请补充上下文使其唯一，或设置 replace_all=true",
                args.path
            )));
        }

        let new_content = if replace_all {
            content.replace(&args.old_string, &args.new_string)
        } else {
            content.replacen(&args.old_string, &args.new_string, 1)
        };
        tokio::fs::write(&path, new_content)
            .await
            .map_err(|e| Error::Tool(format!("写入 {} 失败: {e}", args.path)))?;
        Ok(format!("已编辑 {}（替换 {count} 处）", args.path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn resolves_relative_paths_against_base() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "hello base").unwrap();

        // 相对路径应解析到 base（而非进程 cwd）
        let out = ReadFileTool
            .execute(serde_json::json!({ "path": "a.txt" }), dir.path())
            .await
            .unwrap();
        assert!(out.contains("hello base"));

        // 写入同样相对 base（父目录自动创建）
        WriteFileTool
            .execute(
                serde_json::json!({ "path": "sub/b.txt", "content": "x" }),
                dir.path(),
            )
            .await
            .unwrap();
        assert!(dir.path().join("sub/b.txt").exists());
    }
}
