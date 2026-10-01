//! `sen sessions`：会话列出 / 查看 / 重命名 / 删除。

use anyhow::Result;
use sen_core::util::format_ts;

use super::{clip, open_store};
use crate::SessionsCmd;

pub async fn run(cmd: Option<SessionsCmd>) -> Result<()> {
    let store = open_store()?;
    match cmd.unwrap_or(SessionsCmd::List) {
        SessionsCmd::List => {
            let sessions = store.list_sessions()?;
            if sessions.is_empty() {
                println!("暂无会话。运行 `sen chat` 开始对话。");
                return Ok(());
            }
            println!("{:<38}  {:>4}  {:<16}  标题", "ID", "消息", "更新时间");
            for s in sessions {
                println!(
                    "{:<38}  {:>4}  {:<16}  {}",
                    s.id,
                    s.message_count,
                    format_ts(s.updated_at),
                    if s.title.is_empty() {
                        "(未命名)"
                    } else {
                        &s.title
                    }
                );
            }
            println!("\n继续对话: sen chat --session <id 前缀>");
        }
        SessionsCmd::Show { id } => {
            let sid = resolve_id(&store, &id)?;
            let session = store.get_session(&sid)?.expect("已解析存在的会话");
            println!("会话: {} （{}）", session.title, session.id);
            println!(
                "创建: {}  更新: {}\n",
                format_ts(session.created_at),
                format_ts(session.updated_at)
            );
            for m in store.load_messages(&sid)? {
                match m.role {
                    sen_core::Role::User => println!("\x1b[36m你:\x1b[0m {}", m.content),
                    sen_core::Role::Assistant => {
                        if !m.content.is_empty() {
                            println!("\x1b[32m助手:\x1b[0m {}", m.content);
                        }
                        if let Some(tcs) = &m.tool_calls {
                            for tc in tcs {
                                println!(
                                    "\x1b[2m  → 调用 {}({})\x1b[0m",
                                    tc.name,
                                    clip(&tc.arguments, 120)
                                );
                            }
                        }
                    }
                    sen_core::Role::Tool => {
                        let first = m.content.lines().next().unwrap_or("");
                        println!(
                            "\x1b[2m  ↳ {} => {}\x1b[0m",
                            m.name.as_deref().unwrap_or("tool"),
                            clip(first, 120)
                        );
                    }
                    sen_core::Role::System => {}
                }
                println!();
            }
        }
        SessionsCmd::Rename { id, title } => {
            let sid = resolve_id(&store, &id)?;
            let title = title.join(" ");
            store.rename_session(&sid, &title)?;
            println!("已重命名 {sid} → {title}");
        }
        SessionsCmd::Delete { id } => {
            let sid = resolve_id(&store, &id)?;
            store.delete_session(&sid)?;
            println!("已删除会话 {sid}");
        }
    }
    Ok(())
}

/// 支持完整 id 或唯一前缀。
fn resolve_id(store: &sen_core::Store, want: &str) -> Result<String> {
    let sessions = store.list_sessions()?;
    if let Some(s) = sessions.iter().find(|s| s.id == want) {
        return Ok(s.id.clone());
    }
    let matched: Vec<_> = sessions.iter().filter(|s| s.id.starts_with(want)).collect();
    match matched.as_slice() {
        [one] => Ok(one.id.clone()),
        [] => anyhow::bail!("未找到会话 `{want}`"),
        many => anyhow::bail!(
            "前缀 `{want}` 匹配到 {} 个会话，请提供更长的 id",
            many.len()
        ),
    }
}
