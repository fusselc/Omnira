//! Phase 4 persistence tests: model registry (with missing-file detection),
//! conversations, messages (stream-boundary statuses), and deletion flows.

use omnira_lib::storage::Storage;
use omnira_lib::types::{MessageRole, MessageStatus, ModelStatus};

fn temp_db() -> (Storage, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!("omnira-test-{}.db", uuid::Uuid::new_v4()));
    (Storage::open_at(&path).unwrap(), path)
}

fn write_minimal_gguf(path: &std::path::Path) {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0x4655_4747u32.to_le_bytes()); // GGUF
    bytes.extend_from_slice(&3u32.to_le_bytes()); // version
    bytes.extend_from_slice(&0u64.to_le_bytes()); // tensor count
    bytes.extend_from_slice(&0u64.to_le_bytes()); // metadata kv count
    std::fs::write(path, bytes).unwrap();
}

#[test]
fn model_registry_lifecycle_and_file_status_detection() {
    let (db, dbpath) = temp_db();

    // Register a file that exists.
    let real = std::env::temp_dir().join("omnira-test-model.gguf");
    write_minimal_gguf(&real);
    let m = db
        .add_model("Test Model", &real.display().to_string(), 11, Some(4096))
        .unwrap();
    assert_eq!(m.status, ModelStatus::Ok);
    assert_eq!(m.trained_context_length, Some(4096));

    // Re-adding the same path updates rather than duplicating.
    let again = db
        .add_model("Renamed", &real.display().to_string(), 11, Some(4096))
        .unwrap();
    assert_eq!(db.list_models().unwrap().len(), 1);
    assert_eq!(again.name, "Renamed");
    assert_eq!(again.id, m.id);

    // Missing-file detection: delete the file, status flips to Missing.
    std::fs::remove_file(&real).unwrap();
    let listed = db.list_models().unwrap();
    assert_eq!(listed[0].status, ModelStatus::Missing);

    // Existing-but-corrupt files are surfaced as Invalid, not ready.
    std::fs::write(&real, b"not a gguf").unwrap();
    let listed = db.list_models().unwrap();
    assert_eq!(listed[0].status, ModelStatus::Invalid);
    std::fs::remove_file(&real).unwrap();

    // Display-name rename does not require the file to exist.
    db.rename_model(&m.id, "  Display Name  ").unwrap();
    assert_eq!(db.list_models().unwrap()[0].name, "Display Name");
    assert!(db.rename_model(&m.id, "   ").is_err());
    assert!(db.rename_model(&m.id, "").is_err());

    // Removing the entry never requires the file to exist.
    db.remove_model(&m.id).unwrap();
    assert!(db.list_models().unwrap().is_empty());

    drop(db);
    std::fs::remove_file(&dbpath).ok();
}

#[test]
fn remove_then_add_same_path_keeps_conversation_model_binding() {
    let (db, dbpath) = temp_db();
    let real = std::env::temp_dir().join(format!("omnira-readd-{}.gguf", uuid::Uuid::new_v4()));
    write_minimal_gguf(&real);
    let path = real.display().to_string();

    let original = db.add_model("Bound Model", &path, 11, Some(4096)).unwrap();
    let convo = db
        .create_conversation("uses this model", Some(&original.id))
        .unwrap();

    db.remove_model(&original.id).unwrap();
    assert!(db.get_model(&original.id).unwrap().is_none());

    let again = db.add_model("Bound Model", &path, 11, Some(4096)).unwrap();

    assert_eq!(
        again.id, original.id,
        "re-adding the same GGUF path must reuse the registry id"
    );
    let rebound = db
        .list_conversations()
        .unwrap()
        .into_iter()
        .find(|c| c.id == convo.id)
        .unwrap();
    assert_eq!(rebound.model_id.as_deref(), Some(original.id.as_str()));
    assert!(
        db.get_model(rebound.model_id.as_deref().unwrap())
            .unwrap()
            .is_some(),
        "the conversation's model_id must still resolve in the registry"
    );

    drop(db);
    std::fs::remove_file(&dbpath).ok();
    std::fs::remove_file(&real).ok();
}

#[test]
fn single_model_rebind_repairs_orphaned_conversation() {
    let (db, dbpath) = temp_db();
    let real =
        std::env::temp_dir().join(format!("omnira-rebind-one-{}.gguf", uuid::Uuid::new_v4()));
    write_minimal_gguf(&real);
    let live = db
        .add_model("Only Model", &real.display().to_string(), 11, Some(4096))
        .unwrap();
    let orphan_id = uuid::Uuid::new_v4().to_string();
    let convo = db
        .create_conversation("pre-fix thread", Some(&orphan_id))
        .unwrap();

    let outcome = db.rebind_orphaned_conversations_if_single_model().unwrap();
    assert_eq!(outcome.as_ref().map(|o| o.updated), Some(1));
    assert_eq!(
        outcome.as_ref().map(|o| o.target_model_id.as_str()),
        Some(live.id.as_str())
    );
    let rebound = db
        .list_conversations()
        .unwrap()
        .into_iter()
        .find(|c| c.id == convo.id)
        .unwrap();
    assert_eq!(rebound.model_id.as_deref(), Some(live.id.as_str()));

    let again = db.rebind_orphaned_conversations_if_single_model().unwrap();
    assert_eq!(again.as_ref().map(|o| o.updated), Some(0));

    drop(db);
    std::fs::remove_file(&dbpath).ok();
    std::fs::remove_file(&real).ok();
}

#[test]
fn two_models_leave_orphaned_conversation_untouched() {
    let (db, dbpath) = temp_db();
    let a = std::env::temp_dir().join(format!("omnira-rebind-a-{}.gguf", uuid::Uuid::new_v4()));
    let b = std::env::temp_dir().join(format!("omnira-rebind-b-{}.gguf", uuid::Uuid::new_v4()));
    write_minimal_gguf(&a);
    write_minimal_gguf(&b);
    db.add_model("A", &a.display().to_string(), 11, Some(4096))
        .unwrap();
    db.add_model("B", &b.display().to_string(), 11, Some(4096))
        .unwrap();
    let orphan_id = uuid::Uuid::new_v4().to_string();
    let convo = db
        .create_conversation("ambiguous thread", Some(&orphan_id))
        .unwrap();

    assert!(db
        .rebind_orphaned_conversations_if_single_model()
        .unwrap()
        .is_none());
    let listed = db
        .list_conversations()
        .unwrap()
        .into_iter()
        .find(|c| c.id == convo.id)
        .unwrap();
    assert_eq!(listed.model_id.as_deref(), Some(orphan_id.as_str()));

    drop(db);
    std::fs::remove_file(&dbpath).ok();
    std::fs::remove_file(&a).ok();
    std::fs::remove_file(&b).ok();
}

#[test]
fn add_model_rewrites_conversations_tombstoned_for_the_same_path() {
    let (db, dbpath) = temp_db();
    let real = std::env::temp_dir().join(format!("omnira-tombstone-{}.gguf", uuid::Uuid::new_v4()));
    write_minimal_gguf(&real);
    let path = real.display().to_string();
    let live = db.add_model("Live", &path, 11, Some(4096)).unwrap();
    let old_id = uuid::Uuid::new_v4().to_string();
    let convo = db
        .create_conversation("bound to superseded id", Some(&old_id))
        .unwrap();

    let conn = rusqlite::Connection::open(&dbpath).unwrap();
    let normalized: String = conn
        .query_row("SELECT normalized_path FROM model_path_ids", [], |r| {
            r.get(0)
        })
        .unwrap();
    conn.execute(
        "INSERT INTO model_id_history (id, normalized_path) VALUES (?1, ?2)",
        rusqlite::params![old_id, normalized],
    )
    .unwrap();
    drop(conn);

    db.add_model("Live", &path, 11, Some(4096)).unwrap();
    let rebound = db
        .list_conversations()
        .unwrap()
        .into_iter()
        .find(|c| c.id == convo.id)
        .unwrap();
    assert_eq!(rebound.model_id.as_deref(), Some(live.id.as_str()));

    drop(db);
    std::fs::remove_file(&dbpath).ok();
    std::fs::remove_file(&real).ok();
}

#[test]
fn removing_a_does_not_rebind_its_chat_onto_remaining_model_b() {
    let (db, dbpath) = temp_db();
    let path_a = std::env::temp_dir().join(format!("omnira-keep-a-{}.gguf", uuid::Uuid::new_v4()));
    let path_b = std::env::temp_dir().join(format!("omnira-keep-b-{}.gguf", uuid::Uuid::new_v4()));
    write_minimal_gguf(&path_a);
    write_minimal_gguf(&path_b);
    let model_a = db
        .add_model("A", &path_a.display().to_string(), 11, Some(4096))
        .unwrap();
    let model_b = db
        .add_model("B", &path_b.display().to_string(), 11, Some(4096))
        .unwrap();
    let convo = db
        .create_conversation("bound to A", Some(&model_a.id))
        .unwrap();

    db.remove_model(&model_a.id).unwrap();
    let outcome = db.rebind_orphaned_conversations_if_single_model().unwrap();
    assert_eq!(outcome.as_ref().map(|o| o.updated), Some(0));
    let after_restart = db
        .list_conversations()
        .unwrap()
        .into_iter()
        .find(|c| c.id == convo.id)
        .unwrap();
    assert_eq!(after_restart.model_id.as_deref(), Some(model_a.id.as_str()));
    assert_ne!(after_restart.model_id.as_deref(), Some(model_b.id.as_str()));

    let restored = db
        .add_model("A", &path_a.display().to_string(), 11, Some(4096))
        .unwrap();
    assert_eq!(restored.id, model_a.id);
    let after_readd = db
        .list_conversations()
        .unwrap()
        .into_iter()
        .find(|c| c.id == convo.id)
        .unwrap();
    assert_eq!(after_readd.model_id.as_deref(), Some(model_a.id.as_str()));

    drop(db);
    std::fs::remove_file(&dbpath).ok();
    std::fs::remove_file(&path_a).ok();
    std::fs::remove_file(&path_b).ok();
}

#[test]
fn add_model_treats_equivalent_path_spellings_as_the_same_row() {
    let (db, dbpath) = temp_db();
    let real = std::env::temp_dir().join(format!("omnira-slash-{}.gguf", uuid::Uuid::new_v4()));
    write_minimal_gguf(&real);
    let backslash = real.display().to_string();
    let slash = backslash.replace('\\', "/");
    let first = db.add_model("Slash", &backslash, 11, Some(4096)).unwrap();
    let again = db.add_model("Slash", &slash, 11, Some(4096)).unwrap();
    assert_eq!(again.id, first.id);
    assert_eq!(db.list_models().unwrap().len(), 1);

    drop(db);
    std::fs::remove_file(&dbpath).ok();
    std::fs::remove_file(&real).ok();
}

#[test]
fn conversation_and_message_flows() {
    let (db, dbpath) = temp_db();

    let convo = db.create_conversation("First chat", None).unwrap();

    // Stream-boundary contract: user message persisted before streaming...
    let user = db
        .add_message(
            &convo.id,
            MessageRole::User,
            "Hello",
            MessageStatus::Complete,
        )
        .unwrap();
    // ...and a cancelled generation persists partial content as interrupted.
    db.add_message(
        &convo.id,
        MessageRole::Assistant,
        "Partial resp",
        MessageStatus::Interrupted,
    )
    .unwrap();

    let msgs = db.list_messages(&convo.id).unwrap();
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0].id, user.id);
    assert_eq!(msgs[1].status, MessageStatus::Interrupted);

    // Deleting the conversation cascades to messages.
    db.delete_conversation(&convo.id).unwrap();
    assert!(db.list_messages(&convo.id).unwrap().is_empty());
    assert!(db.list_conversations().unwrap().is_empty());

    // Clear-all works across multiple conversations.
    let c1 = db.create_conversation("A", None).unwrap();
    db.create_conversation("B", None).unwrap();
    db.add_message(&c1.id, MessageRole::User, "x", MessageStatus::Complete)
        .unwrap();
    db.clear_conversations().unwrap();
    assert!(db.list_conversations().unwrap().is_empty());

    drop(db);
    std::fs::remove_file(&dbpath).ok();
}
