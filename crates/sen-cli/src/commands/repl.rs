//! `sen chat`：交互式 REPL（rustyline 行编辑 + 流式输出）。
//!
//! 架构：stdin 读取线程（rustyline 非线程安全，不能跨 await 持有）
//! 通过 channel 把用户输入送给异步主循环，主循环负责运行 Agent 并渲染事件流。

use anyhow::{Context, Result};
use sen_core::TurnMode;
use tokio::sync::mpsc;

use super::{build_agent, print_stream, resolve_session};

pub async fn run(provider: Option<String>, session: Option<String>) -> Result<()> {
    let agent = build_agent(provider.as_deref())?;
    let mut session_id = resolve_session(&agent, session.as_deref())?;

    println!(
        "SenAgent v{} | provider: {} | model: {} | wire_api: {}",
        env!("CARGO_PKG_VERSION"),
        agent.provider().name(),
        agent.provider().model(),
        agent.provider().wire_api()
    );
    println!("会话: {session_id}");
    println!("命令: /new 新建会话  /session 显示当前会话  /exit 退出\n");

    let (tx, mut rx) = mpsc::channel::<String>(8);
    let history_path = sen_core::paths::history_path();
    std::thread::spawn(move || reader_thread(tx, history_path));

    while let Some(line) = rx.recv().await {
        match line.as_str() {
            "/exit" | "/quit" => break,
            "/new" => {
                session_id = agent
                    .store()
                    .create_session("", None)
                    .context("创建会话失败")?
                    .id;
                println!("已切换到新会话: {session_id}\n");
                continue;
            }
            "/session" => {
                println!("当前会话: {session_id}\n");
                continue;
            }
            _ => {}
        }
        println!();
        print_stream(agent.run_turn(&session_id, &line, TurnMode::Normal)).await;
        println!();
    }
    println!("再见！");
    Ok(())
}

/// 输入读取线程：优先 rustyline（行编辑 + 历史），失败时退化为逐行读取。
fn reader_thread(tx: mpsc::Sender<String>, history_path: std::path::PathBuf) {
    use rustyline::error::ReadlineError;

    let mut rl = match rustyline::DefaultEditor::new() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("提示: 行编辑器初始化失败（{e}），使用基础输入模式");
            use std::io::BufRead;
            for line in std::io::stdin().lock().lines().map_while(|l| l.ok()) {
                if tx.blocking_send(line).is_err() {
                    return;
                }
            }
            return;
        }
    };
    let _ = rl.load_history(&history_path);
    loop {
        match rl.readline("you> ") {
            Ok(line) => {
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }
                let _ = rl.add_history_entry(line.as_str());
                if tx.blocking_send(line).is_err() {
                    break;
                }
            }
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(_) => break,
        }
    }
    let _ = rl.save_history(&history_path);
}
