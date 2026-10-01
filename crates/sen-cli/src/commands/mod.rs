//! CLI 子命令实现与共用辅助。

pub mod config_cmd;
pub mod repl;
pub mod serve;
pub mod sessions;
pub mod skill;

use anyhow::{Context, Result};
use futures_util::StreamExt;
use sen_core::{Agent, AgentEvent, Config, Store};

/// 按配置构建 Agent（进程内直连内核）。
pub fn build_agent(provider: Option<&str>) -> Result<Agent> {
    let config = Config::load().context("加载配置失败")?;
    let cwd = std::env::current_dir().context("获取当前目录失败")?;
    Agent::from_config(config, provider, cwd).context("初始化 Agent 失败")
}

/// 打开会话存储（无需 provider 的命令使用）。
pub fn open_store() -> Result<Store> {
    let _ = sen_core::paths::ensure_base_dir();
    Store::open(&sen_core::paths::db_path()).context("打开会话数据库失败")
}

/// 解析 `--session`：完整 id / 唯一前缀；未指定则新建会话。
pub fn resolve_session(agent: &Agent, want: Option<&str>) -> Result<String> {
    let Some(w) = want else {
        return Ok(agent.store().create_session("")?.id);
    };
    let sessions = agent.store().list_sessions()?;
    if let Some(s) = sessions.iter().find(|s| s.id == w) {
        return Ok(s.id.clone());
    }
    let matched: Vec<_> = sessions.iter().filter(|s| s.id.starts_with(w)).collect();
    match matched.as_slice() {
        [one] => Ok(one.id.clone()),
        [] => anyhow::bail!("未找到会话 `{w}`（运行 `sen sessions` 查看全部会话）"),
        many => anyhow::bail!(
            "会话前缀 `{w}` 匹配到 {} 个会话，请提供更长的 id",
            many.len()
        ),
    }
}

/// `sen run "..."]`：单次问答。
pub async fn run_once(
    provider: Option<String>,
    session: Option<String>,
    prompt: String,
) -> Result<()> {
    let prompt = prompt.trim().to_string();
    if prompt.is_empty() {
        anyhow::bail!("请提供问题内容，例如: sen run \"总结这个项目\"");
    }
    let agent = build_agent(provider.as_deref())?;
    let session_id = resolve_session(&agent, session.as_deref())?;
    println!("\x1b[2msession: {session_id}\x1b[0m");
    let ok = print_stream(agent.run_turn(&session_id, &prompt)).await;
    if !ok {
        std::process::exit(1);
    }
    Ok(())
}

/// 消费事件流并输出到终端；返回是否以 Done 正常结束。
pub async fn print_stream<S: futures_util::Stream<Item = AgentEvent>>(stream: S) -> bool {
    use std::io::Write as _;

    futures_util::pin_mut!(stream);
    let mut in_reasoning = false;
    let mut at_line_start = true;
    let mut ok = false;

    while let Some(ev) = stream.next().await {
        match ev {
            AgentEvent::Token { delta } => {
                if in_reasoning {
                    print!("\x1b[0m");
                    in_reasoning = false;
                }
                print!("{delta}");
                at_line_start = delta.ends_with('\n');
                let _ = std::io::stdout().flush();
            }
            AgentEvent::Reasoning { delta } => {
                if !in_reasoning {
                    print!("\x1b[2m[推理] ");
                    in_reasoning = true;
                }
                print!("{delta}");
                at_line_start = delta.ends_with('\n');
                let _ = std::io::stdout().flush();
            }
            AgentEvent::ToolCall { name, arguments } => {
                if in_reasoning {
                    print!("\x1b[0m");
                    in_reasoning = false;
                }
                if !at_line_start {
                    println!();
                }
                println!(
                    "\x1b[36m→ {name}\x1b[0m \x1b[2m{}\x1b[0m",
                    clip(&arguments, 160)
                );
                at_line_start = true;
            }
            AgentEvent::ToolResult {
                name,
                output,
                is_error,
            } => {
                if !at_line_start {
                    println!();
                }
                let color = if is_error { "31" } else { "32" };
                let first = output.lines().next().unwrap_or("(无输出)");
                println!(
                    "\x1b[{color}m↳ {name}\x1b[0m \x1b[2m{}\x1b[0m",
                    clip(first, 200)
                );
                at_line_start = true;
            }
            AgentEvent::Done { usage } => {
                if in_reasoning {
                    print!("\x1b[0m");
                }
                if !at_line_start {
                    println!();
                }
                if let Some(u) = usage {
                    println!(
                        "\x1b[2m[tokens] in {} / out {}\x1b[0m",
                        u.input_tokens, u.output_tokens
                    );
                }
                ok = true;
            }
            AgentEvent::Error { message } => {
                if in_reasoning {
                    print!("\x1b[0m");
                }
                if !at_line_start {
                    println!();
                }
                eprintln!("\x1b[31m错误: {message}\x1b[0m");
            }
        }
    }
    ok
}

/// 单行化并截断（终端展示用）。
pub fn clip(s: &str, max: usize) -> String {
    let one = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if one.chars().count() <= max {
        one
    } else {
        format!("{}…", one.chars().take(max).collect::<String>())
    }
}
