//! SQLite persistence (docs/data-ownership-and-storage.md section 2).
//! Owns conversations, messages, and the model registry.

use std::sync::Mutex;

use rusqlite::Connection;

use crate::errors::AppError;
use crate::paths;
use crate::types::{
    Conversation, Message, MessageRole, MessageStatus, ModelEntry, ModelStatus,
};

pub struct Storage {
    conn: Mutex<Connection>,
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS models (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    file_size_bytes INTEGER NOT NULL,
    trained_context_length INTEGER,
    last_used_at TEXT,
    added_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS conversations (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    model_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
    content TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'complete' CHECK (status IN ('complete', 'interrupted')),
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_messages_conversation
    ON messages(conversation_id, created_at);
";

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

impl Storage {
    pub fn open() -> Result<Self, AppError> {
        Self::open_at(&paths::db_path())
    }

    pub fn open_at(path: &std::path::Path) -> Result<Self, AppError> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn with<T>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, rusqlite::Error>,
    ) -> Result<T, AppError> {
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());
        Ok(f(&conn)?)
    }

    // -- Models --------------------------------------------------------------

    pub fn add_model(
        &self,
        name: &str,
        path: &str,
        file_size_bytes: u64,
        trained_context_length: Option<u64>,
    ) -> Result<ModelEntry, AppError> {
        let id = new_id();
        let added_at = now();
        self.with(|c| {
            c.execute(
                "INSERT INTO models (id, name, path, file_size_bytes, trained_context_length, added_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(path) DO UPDATE SET
                    name = excluded.name,
                    file_size_bytes = excluded.file_size_bytes,
                    trained_context_length = excluded.trained_context_length",
                rusqlite::params![id, name, path, file_size_bytes, trained_context_length, added_at],
            )?;
            Ok(())
        })?;
        // Fetch by path so the ON CONFLICT update path returns the existing row.
        let models = self.list_models()?;
        models
            .into_iter()
            .find(|m| m.path == path)
            .ok_or_else(|| AppError::from(rusqlite::Error::QueryReturnedNoRows))
    }

    pub fn list_models(&self) -> Result<Vec<ModelEntry>, AppError> {
        self.with(|c| {
            let mut stmt = c.prepare(
                "SELECT id, name, path, file_size_bytes, trained_context_length, last_used_at, added_at
                 FROM models ORDER BY added_at DESC",
            )?;
            let rows = stmt.query_map([], |r| {
                let path: String = r.get(2)?;
                let status = if std::path::Path::new(&path).is_file() {
                    ModelStatus::Ok
                } else {
                    ModelStatus::Missing
                };
                Ok(ModelEntry {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    path,
                    file_size_bytes: r.get::<_, i64>(3)? as u64,
                    trained_context_length: r.get::<_, Option<i64>>(4)?.map(|v| v as u64),
                    last_used_at: r.get(5)?,
                    added_at: r.get(6)?,
                    status,
                })
            })?;
            rows.collect()
        })
    }

    pub fn get_model(&self, id: &str) -> Result<Option<ModelEntry>, AppError> {
        Ok(self.list_models()?.into_iter().find(|m| m.id == id))
    }

    pub fn touch_model(&self, id: &str) -> Result<(), AppError> {
        let ts = now();
        self.with(|c| {
            c.execute(
                "UPDATE models SET last_used_at = ?1 WHERE id = ?2",
                rusqlite::params![ts, id],
            )?;
            Ok(())
        })
    }

    /// Removes only the registry entry. Never touches the model file.
    pub fn remove_model(&self, id: &str) -> Result<(), AppError> {
        self.with(|c| {
            c.execute("DELETE FROM models WHERE id = ?1", rusqlite::params![id])?;
            Ok(())
        })
    }

    // -- Conversations ---------------------------------------------------------

    pub fn create_conversation(
        &self,
        title: &str,
        model_id: Option<&str>,
    ) -> Result<Conversation, AppError> {
        let convo = Conversation {
            id: new_id(),
            title: title.to_string(),
            model_id: model_id.map(|s| s.to_string()),
            created_at: now(),
            updated_at: now(),
        };
        self.with(|c| {
            c.execute(
                "INSERT INTO conversations (id, title, model_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    convo.id,
                    convo.title,
                    convo.model_id,
                    convo.created_at,
                    convo.updated_at
                ],
            )?;
            Ok(())
        })?;
        Ok(convo)
    }

    pub fn list_conversations(&self) -> Result<Vec<Conversation>, AppError> {
        self.with(|c| {
            let mut stmt = c.prepare(
                "SELECT id, title, model_id, created_at, updated_at
                 FROM conversations ORDER BY updated_at DESC",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok(Conversation {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    model_id: r.get(2)?,
                    created_at: r.get(3)?,
                    updated_at: r.get(4)?,
                })
            })?;
            rows.collect()
        })
    }

    pub fn rename_conversation(&self, id: &str, title: &str) -> Result<(), AppError> {
        let ts = now();
        self.with(|c| {
            c.execute(
                "UPDATE conversations SET title = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![title, ts, id],
            )?;
            Ok(())
        })
    }

    pub fn set_conversation_model(&self, id: &str, model_id: &str) -> Result<(), AppError> {
        let ts = now();
        self.with(|c| {
            c.execute(
                "UPDATE conversations SET model_id = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![model_id, ts, id],
            )?;
            Ok(())
        })
    }

    pub fn delete_conversation(&self, id: &str) -> Result<(), AppError> {
        self.with(|c| {
            c.execute("DELETE FROM conversations WHERE id = ?1", rusqlite::params![id])?;
            Ok(())
        })
    }

    pub fn clear_conversations(&self) -> Result<(), AppError> {
        self.with(|c| {
            c.execute_batch("DELETE FROM messages; DELETE FROM conversations;")?;
            Ok(())
        })
    }

    // -- Messages ----------------------------------------------------------------

    /// Persist a message. Per the stream-boundary contract
    /// (docs/data-ownership-and-storage.md), user messages are stored BEFORE
    /// streaming starts; assistant messages are stored on completion or, with
    /// `MessageStatus::Interrupted`, on cancellation.
    pub fn add_message(
        &self,
        conversation_id: &str,
        role: MessageRole,
        content: &str,
        status: MessageStatus,
    ) -> Result<Message, AppError> {
        let msg = Message {
            id: new_id(),
            conversation_id: conversation_id.to_string(),
            role,
            content: content.to_string(),
            status,
            created_at: now(),
        };
        let role_s = match role {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
        };
        let status_s = match status {
            MessageStatus::Complete => "complete",
            MessageStatus::Interrupted => "interrupted",
        };
        let ts = now();
        self.with(|c| {
            c.execute(
                "INSERT INTO messages (id, conversation_id, role, content, status, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![msg.id, msg.conversation_id, role_s, msg.content, status_s, msg.created_at],
            )?;
            c.execute(
                "UPDATE conversations SET updated_at = ?1 WHERE id = ?2",
                rusqlite::params![ts, conversation_id],
            )?;
            Ok(())
        })?;
        Ok(msg)
    }

    pub fn list_messages(&self, conversation_id: &str) -> Result<Vec<Message>, AppError> {
        self.with(|c| {
            let mut stmt = c.prepare(
                "SELECT id, conversation_id, role, content, status, created_at
                 FROM messages WHERE conversation_id = ?1 ORDER BY created_at ASC",
            )?;
            let rows = stmt.query_map([conversation_id], |r| {
                let role_s: String = r.get(2)?;
                let status_s: String = r.get(4)?;
                Ok(Message {
                    id: r.get(0)?,
                    conversation_id: r.get(1)?,
                    role: if role_s == "user" {
                        MessageRole::User
                    } else {
                        MessageRole::Assistant
                    },
                    content: r.get(3)?,
                    status: if status_s == "interrupted" {
                        MessageStatus::Interrupted
                    } else {
                        MessageStatus::Complete
                    },
                    created_at: r.get(5)?,
                })
            })?;
            rows.collect()
        })
    }
}
