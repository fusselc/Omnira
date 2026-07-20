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

    // Missing-file detection: delete the file, status flips to Missing.
    std::fs::remove_file(&real).unwrap();
    let listed = db.list_models().unwrap();
    assert_eq!(listed[0].status, ModelStatus::Missing);

    // Existing-but-corrupt files are surfaced as Invalid, not ready.
    std::fs::write(&real, b"not a gguf").unwrap();
    let listed = db.list_models().unwrap();
    assert_eq!(listed[0].status, ModelStatus::Invalid);
    std::fs::remove_file(&real).unwrap();

    // Removing the entry never requires the file to exist.
    db.remove_model(&m.id).unwrap();
    assert!(db.list_models().unwrap().is_empty());

    drop(db);
    std::fs::remove_file(&dbpath).ok();
}

#[test]
fn conversation_and_message_flows() {
    let (db, dbpath) = temp_db();

    let convo = db.create_conversation("First chat", None).unwrap();

    // Stream-boundary contract: user message persisted before streaming...
    let user = db
        .add_message(&convo.id, MessageRole::User, "Hello", MessageStatus::Complete)
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
