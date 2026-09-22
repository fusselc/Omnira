//! SQLite persistence (docs/data-ownership-and-storage.md section 2).
//! Owns conversations, messages, and the model registry.

use std::sync::Mutex;

use rusqlite::{Connection, OptionalExtension};

use crate::errors::{AppError, ErrorCode};
use crate::gguf;
use crate::paths;
use crate::types::{Conversation, Message, MessageRole, MessageStatus, ModelEntry, ModelStatus};

pub struct Storage {
    conn: Mutex<Connection>,
}

/// Result of the conservative single-model orphan repair. `None` from
/// `rebind_orphaned_conversations_if_single_model` means zero or several
/// registry models, so nothing was changed.
pub struct OrphanRebind {
    pub updated: u64,
    pub target_model_id: String,
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

CREATE TABLE IF NOT EXISTS model_path_ids (
    normalized_path TEXT PRIMARY KEY,
    id TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS model_id_history (
    id TEXT PRIMARY KEY,
    normalized_path TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_model_id_history_path
    ON model_id_history(normalized_path);
";

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn model_status(path: &str) -> ModelStatus {
    let path = std::path::Path::new(path);
    if !path.is_file() {
        return ModelStatus::Missing;
    }
    match gguf::inspect(path) {
        Ok(_) => ModelStatus::Ok,
        Err(_) => ModelStatus::Invalid,
    }
}

/// Comparison key so the same GGUF file keeps one registry UUID across
/// Remove then Add, even when separators or drive-letter case differ.
fn normalize_model_path(path: &str) -> String {
    let mut normalized = path.trim().replace('/', "\\");
    while normalized.contains("\\\\") {
        normalized = normalized.replace("\\\\", "\\");
    }
    #[cfg(windows)]
    {
        normalized.make_ascii_lowercase();
    }
    normalized
}

impl Storage {
    pub fn open() -> Result<Self, AppError> {
        Self::open_at(&paths::db_path())
    }

    pub fn open_at(path: &std::path::Path) -> Result<Self, AppError> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        conn.execute_batch(SCHEMA)?;
        let storage = Self {
            conn: Mutex::new(conn),
        };
        storage.backfill_model_path_ids()?;
        storage.backfill_model_id_history()?;
        Ok(storage)
    }

    fn with<T>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, rusqlite::Error>,
    ) -> Result<T, AppError> {
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());
        Ok(f(&conn)?)
    }

    fn backfill_model_path_ids(&self) -> Result<(), AppError> {
        self.with(|c| {
            let rows: Vec<(String, String)> = {
                let mut stmt = c.prepare("SELECT id, path FROM models")?;
                let mapped = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
                mapped.collect::<Result<_, _>>()?
            };
            for (id, path) in rows {
                let normalized = normalize_model_path(&path);
                c.execute(
                    "INSERT OR IGNORE INTO model_path_ids (normalized_path, id) VALUES (?1, ?2)",
                    rusqlite::params![normalized, id],
                )?;
                c.execute(
                    "INSERT OR IGNORE INTO model_id_history (id, normalized_path) VALUES (?1, ?2)",
                    rusqlite::params![id, normalized],
                )?;
            }
            Ok(())
        })
    }

    fn backfill_model_id_history(&self) -> Result<(), AppError> {
        self.with(|c| {
            c.execute_batch(
                "INSERT OR IGNORE INTO model_id_history (id, normalized_path)
                 SELECT id, normalized_path FROM model_path_ids",
            )?;
            Ok(())
        })
    }

    fn record_id_history(&self, normalized: &str, id: &str) -> Result<(), AppError> {
        self.with(|c| {
            c.execute(
                "INSERT OR IGNORE INTO model_id_history (id, normalized_path) VALUES (?1, ?2)",
                rusqlite::params![id, normalized],
            )?;
            Ok(())
        })
    }

    fn rebind_conversations_for_path(
        &self,
        normalized: &str,
        new_id: &str,
    ) -> Result<(), AppError> {
        let ts = now();
        self.with(|c| {
            c.execute(
                "UPDATE conversations SET model_id = ?1, updated_at = ?2
                 WHERE model_id IN (
                    SELECT id FROM model_id_history WHERE normalized_path = ?3
                 )
                 AND model_id != ?1",
                rusqlite::params![new_id, ts, normalized],
            )?;
            Ok(())
        })
    }

    fn remember_model_identity(&self, normalized: &str, id: &str) -> Result<(), AppError> {
        self.remember_path_id(normalized, id)?;
        self.record_id_history(normalized, id)?;
        self.rebind_conversations_for_path(normalized, id)
    }

    fn live_model_id_for_normalized(&self, normalized: &str) -> Result<Option<String>, AppError> {
        let rows: Vec<(String, String)> = self.with(|c| {
            let mut stmt = c.prepare("SELECT id, path FROM models")?;
            let mapped = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            mapped.collect()
        })?;
        Ok(rows.into_iter().find_map(|(id, path)| {
            (normalize_model_path(&path) == normalized).then_some(id)
        }))
    }

    fn stable_id_for_path(&self, normalized: &str) -> Result<String, AppError> {
        self.with(|c| {
            let existing: Option<String> = c
                .query_row(
                    "SELECT id FROM model_path_ids WHERE normalized_path = ?1",
                    rusqlite::params![normalized],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(id) = existing {
                return Ok(id);
            }
            let id = new_id();
            c.execute(
                "INSERT INTO model_path_ids (normalized_path, id) VALUES (?1, ?2)",
                rusqlite::params![normalized, id],
            )?;
            Ok(id)
        })
    }

    fn remember_path_id(&self, normalized: &str, id: &str) -> Result<(), AppError> {
        self.with(|c| {
            c.execute(
                "INSERT OR REPLACE INTO model_path_ids (normalized_path, id) VALUES (?1, ?2)",
                rusqlite::params![normalized, id],
            )?;
            Ok(())
        })
    }

    // -- Models --------------------------------------------------------------

    pub fn add_model(
        &self,
        name: &str,
        path: &str,
        file_size_bytes: u64,
        trained_context_length: Option<u64>,
    ) -> Result<ModelEntry, AppError> {
        let normalized = normalize_model_path(path);
        if let Some(existing_id) = self.live_model_id_for_normalized(&normalized)? {
            self.with(|c| {
                c.execute(
                    "UPDATE models SET name = ?1, path = ?2, file_size_bytes = ?3, trained_context_length = ?4
                     WHERE id = ?5",
                    rusqlite::params![
                        name,
                        path,
                        file_size_bytes,
                        trained_context_length,
                        existing_id
                    ],
                )?;
                Ok(())
            })?;
            self.remember_model_identity(&normalized, &existing_id)?;
            return self
                .get_model(&existing_id)?
                .ok_or_else(|| AppError::from(rusqlite::Error::QueryReturnedNoRows));
        }

        let id = self.stable_id_for_path(&normalized)?;
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
        let entry = match self.get_model(&id)? {
            Some(model) => model,
            None => self
                .list_models()?
                .into_iter()
                .find(|m| m.path == path)
                .ok_or_else(|| AppError::from(rusqlite::Error::QueryReturnedNoRows))?,
        };
        self.remember_model_identity(&normalized, &entry.id)?;
        Ok(entry)
    }

    pub fn list_models(&self) -> Result<Vec<ModelEntry>, AppError> {
        self.with(|c| {
            let mut stmt = c.prepare(
                "SELECT id, name, path, file_size_bytes, trained_context_length, last_used_at, added_at
                 FROM models ORDER BY added_at DESC",
            )?;
            let rows = stmt.query_map([], |r| {
                let path: String = r.get(2)?;
                let status = model_status(&path);
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
    /// The path→id mapping is kept so a later add of the same file reuses
    /// this id and existing conversations stay bound. The removed id is
    /// tombstoned in `model_id_history` so a later add for that path can
    /// rewrite conversations still pointing at it.
    pub fn remove_model(&self, id: &str) -> Result<(), AppError> {
        let path: Option<String> = self.with(|c| {
            c.query_row(
                "SELECT path FROM models WHERE id = ?1",
                rusqlite::params![id],
                |r| r.get(0),
            )
            .optional()
        })?;
        self.with(|c| {
            c.execute("DELETE FROM models WHERE id = ?1", rusqlite::params![id])?;
            Ok(())
        })?;
        if let Some(path) = path {
            self.record_id_history(&normalize_model_path(&path), id)?;
        }
        Ok(())
    }

    /// When exactly one model is registered, rebind conversations whose
    /// `model_id` is missing from the registry **and** has no
    /// `model_id_history` row. Ids with history belong to a known path and
    /// are left alone so a later re-add of that path can restore them.
    /// No-op if there are zero or several models. Safe to run on every startup.
    pub fn rebind_orphaned_conversations_if_single_model(
        &self,
    ) -> Result<Option<OrphanRebind>, AppError> {
        let ids: Vec<String> = self.with(|c| {
            let mut stmt = c.prepare("SELECT id FROM models")?;
            let rows = stmt.query_map([], |r| r.get(0))?;
            rows.collect()
        })?;
        let mut ids = ids.into_iter();
        let Some(target_model_id) = ids.next() else {
            return Ok(None);
        };
        if ids.next().is_some() {
            return Ok(None);
        }
        let ts = now();
        let updated = self.with(|c| {
            let n = c.execute(
                "UPDATE conversations SET model_id = ?1, updated_at = ?2
                 WHERE model_id IS NOT NULL
                   AND model_id NOT IN (SELECT id FROM models)
                   AND model_id NOT IN (SELECT id FROM model_id_history)",
                rusqlite::params![target_model_id, ts],
            )?;
            Ok(n as u64)
        })?;
        Ok(Some(OrphanRebind {
            updated,
            target_model_id,
        }))
    }

    /// Renames the registry display name only. Never renames or moves the file.
    pub fn rename_model(&self, id: &str, name: &str) -> Result<(), AppError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(AppError::new(
                ErrorCode::UnknownRuntimeError,
                Some("Model name cannot be empty".into()),
            ));
        }
        self.with(|c| {
            c.execute(
                "UPDATE models SET name = ?1 WHERE id = ?2",
                rusqlite::params![trimmed, id],
            )?;
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
            c.execute(
                "DELETE FROM conversations WHERE id = ?1",
                rusqlite::params![id],
            )?;
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
                rusqlite::params![
                    msg.id,
                    msg.conversation_id,
                    role_s,
                    msg.content,
                    status_s,
                    msg.created_at
                ],
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
