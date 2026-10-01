//! SQLite 存储：sessions / messages 表。
//!
//! 操作为本地毫秒级小事务，使用同步互斥锁保护单个连接即可，
//! 无需引入连接池（个人 Agent 场景下并发极低）。

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::memory::{Message, Role, ToolCall};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub message_count: i64,
}

pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    pub fn open(path: &Path) -> Result<Store> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(Error::Io)?;
        }
        let conn = Connection::open(path).map_err(map_db)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL DEFAULT '',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                tool_calls TEXT,
                tool_call_id TEXT,
                name TEXT,
                created_at INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_messages_session
                ON messages(session_id, created_at);",
        )
        .map_err(map_db)?;
        Ok(Store {
            conn: Mutex::new(conn),
        })
    }

    pub fn create_session(&self, title: &str) -> Result<Session> {
        let now = crate::util::now_ms();
        let id = uuid::Uuid::new_v4().to_string();
        let conn = self.lock();
        conn.execute(
            "INSERT INTO sessions (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, title, now, now],
        )
        .map_err(map_db)?;
        Ok(Session {
            id,
            title: title.to_string(),
            created_at: now,
            updated_at: now,
            message_count: 0,
        })
    }

    pub fn list_sessions(&self) -> Result<Vec<Session>> {
        let conn = self.lock();
        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.title, s.created_at, s.updated_at,
                        (SELECT COUNT(*) FROM messages m WHERE m.session_id = s.id)
                 FROM sessions s ORDER BY s.updated_at DESC",
            )
            .map_err(map_db)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Session {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                    message_count: row.get(4)?,
                })
            })
            .map_err(map_db)?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(map_db)
    }

    pub fn get_session(&self, id: &str) -> Result<Option<Session>> {
        let conn = self.lock();
        conn.query_row(
            "SELECT s.id, s.title, s.created_at, s.updated_at,
                    (SELECT COUNT(*) FROM messages m WHERE m.session_id = s.id)
             FROM sessions s WHERE s.id = ?1",
            params![id],
            |row| {
                Ok(Session {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                    message_count: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(map_db)
    }

    pub fn rename_session(&self, id: &str, title: &str) -> Result<()> {
        let conn = self.lock();
        conn.execute(
            "UPDATE sessions SET title = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, title, crate::util::now_ms()],
        )
        .map_err(map_db)?;
        Ok(())
    }

    pub fn delete_session(&self, id: &str) -> Result<()> {
        let conn = self.lock();
        conn.execute("DELETE FROM messages WHERE session_id = ?1", params![id])
            .map_err(map_db)?;
        conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])
            .map_err(map_db)?;
        Ok(())
    }

    /// 会话标题为空时，用首条用户消息生成标题。
    pub fn maybe_set_title(&self, id: &str, text: &str) -> Result<()> {
        let title: String = text
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .chars()
            .take(40)
            .collect();
        if title.is_empty() {
            return Ok(());
        }
        let conn = self.lock();
        conn.execute(
            "UPDATE sessions SET title = ?2, updated_at = ?3 WHERE id = ?1 AND title = ''",
            params![id, title, crate::util::now_ms()],
        )
        .map_err(map_db)?;
        Ok(())
    }

    pub fn append_message(&self, session_id: &str, msg: &Message) -> Result<()> {
        let tool_calls = match &msg.tool_calls {
            Some(tcs) => Some(serde_json::to_string(tcs).map_err(Error::Json)?),
            None => None,
        };
        let conn = self.lock();
        conn.execute(
            "INSERT INTO messages (id, session_id, role, content, tool_calls, tool_call_id, name, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                msg.id,
                session_id,
                msg.role.as_str(),
                msg.content,
                tool_calls,
                msg.tool_call_id,
                msg.name,
                msg.created_at
            ],
        )
        .map_err(map_db)?;
        conn.execute(
            "UPDATE sessions SET updated_at = ?2 WHERE id = ?1",
            params![session_id, crate::util::now_ms()],
        )
        .map_err(map_db)?;
        Ok(())
    }

    pub fn load_messages(&self, session_id: &str) -> Result<Vec<Message>> {
        let conn = self.lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, role, content, tool_calls, tool_call_id, name, created_at
                 FROM messages WHERE session_id = ?1 ORDER BY created_at ASC, rowid ASC",
            )
            .map_err(map_db)?;
        let rows = stmt
            .query_map(params![session_id], |row| {
                let tc_json: Option<String> = row.get(3)?;
                let tool_calls = tc_json.and_then(|s| serde_json::from_str::<Vec<ToolCall>>(&s).ok());
                Ok(Message {
                    id: row.get(0)?,
                    role: Role::parse(&row.get::<_, String>(1)?),
                    content: row.get(2)?,
                    tool_calls,
                    tool_call_id: row.get(4)?,
                    name: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })
            .map_err(map_db)?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(map_db)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }
}

fn map_db(e: rusqlite::Error) -> Error {
    Error::Storage(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("test.db")).unwrap();
        (dir, store)
    }

    #[test]
    fn session_crud_and_messages() {
        let (_dir, store) = temp_store();
        let s = store.create_session("").unwrap();
        store.maybe_set_title(&s.id, "帮我重构这段代码").unwrap();

        let user = Message::user("帮我重构这段代码");
        store.append_message(&s.id, &user).unwrap();
        let call = ToolCall {
            id: "call_1".into(),
            name: "read_file".into(),
            arguments: "{\"path\":\"a.rs\"}".into(),
        };
        let assistant = Message::assistant("好的", Some(vec![call]));
        store.append_message(&s.id, &assistant).unwrap();
        let tool = Message::tool_result("call_1", "read_file", "fn main() {}");
        store.append_message(&s.id, &tool).unwrap();

        let msgs = store.load_messages(&s.id).unwrap();
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[1].tool_calls.as_ref().unwrap()[0].name, "read_file");
        assert_eq!(msgs[2].tool_call_id.as_deref(), Some("call_1"));

        let got = store.get_session(&s.id).unwrap().unwrap();
        assert_eq!(got.title, "帮我重构这段代码");
        assert_eq!(got.message_count, 3);

        let list = store.list_sessions().unwrap();
        assert_eq!(list.len(), 1);

        store.rename_session(&s.id, "新标题").unwrap();
        assert_eq!(store.get_session(&s.id).unwrap().unwrap().title, "新标题");

        store.delete_session(&s.id).unwrap();
        assert!(store.list_sessions().unwrap().is_empty());
        assert!(store.load_messages(&s.id).unwrap().is_empty());
    }
}
