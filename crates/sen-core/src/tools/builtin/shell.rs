//! shell 工具：在本机执行命令。

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

use crate::error::{Error, Result};
use crate::tools::{resolve_path, schema_of, Tool};
use crate::util::truncate;

pub struct ShellTool;

#[derive(Deserialize, JsonSchema)]
struct Args {
    /// 要执行的命令
    command: String,
    /// 工作目录（可选，默认当前目录）
    #[serde(default)]
    cwd: Option<String>,
    /// 超时秒数（默认 120）
    #[serde(default)]
    timeout_secs: Option<u64>,
}

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &'static str {
        "shell"
    }

    fn description(&self) -> &'static str {
        "在用户本机执行 shell 命令并返回 stdout/stderr/退出码。适用于构建、测试、git、环境探查等。命令超时（默认 120 秒）后会被终止。"
    }

    fn parameters_schema(&self) -> Value {
        schema_of::<Args>()
    }

    async fn execute(&self, args: Value, base: &Path) -> Result<String> {
        let args: Args = serde_json::from_value(args).map_err(|e| Error::Tool(format!("参数不合法: {e}")))?;

        let mut cmd = if cfg!(windows) {
            let mut c = tokio::process::Command::new("cmd");
            c.arg("/C").arg(&args.command);
            c
        } else {
            let mut c = tokio::process::Command::new("bash");
            c.arg("-lc").arg(&args.command);
            c
        };
        let cwd = args
            .cwd
            .as_deref()
            .map(|c| resolve_path(base, c))
            .unwrap_or_else(|| base.to_path_buf());
        cmd.current_dir(cwd);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let timeout = Duration::from_secs(args.timeout_secs.unwrap_or(120));
        let output = match tokio::time::timeout(timeout, cmd.output()).await {
            Ok(Ok(o)) => o,
            Ok(Err(e)) => return Err(Error::Tool(format!("无法执行命令: {e}"))),
            Err(_) => {
                return Err(Error::Tool(format!(
                    "命令超时（{}s 未完成，已终止）",
                    timeout.as_secs()
                )))
            }
        };

        let stdout = truncate(&String::from_utf8_lossy(&output.stdout), 20_000);
        let stderr = truncate(&String::from_utf8_lossy(&output.stderr), 10_000);
        let code = output
            .status
            .code()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "signal".into());

        let mut out = format!("exit_code: {code}\n");
        if !stdout.trim().is_empty() {
            out.push_str(&format!("stdout:\n{stdout}\n"));
        }
        if !stderr.trim().is_empty() {
            out.push_str(&format!("stderr:\n{stderr}\n"));
        }
        if stdout.trim().is_empty() && stderr.trim().is_empty() {
            out.push_str("(no output)\n");
        }
        Ok(out)
    }
}
