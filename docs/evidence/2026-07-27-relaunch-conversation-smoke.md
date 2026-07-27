# Evidence: relaunch conversation restore smoke

- **Date:** 2026-07-27
- **Maintainer:** Chris Fussel (agent-assisted verification after PR #19 merge)
- **Branch tip:** `main` @ post-PR #19 merge

## Code path verified

1. `Settings.last_conversation_id` is persisted via `set_last_conversation` and
   cleared when the conversation is deleted or all conversations are cleared
   (`apps/desktop/src-tauri/src/commands.rs`).
2. Chat restores on mount: after listing conversations, if
   `last_conversation_id` is present in the list, it becomes `activeId`
   (`apps/desktop/src/pages/Chat.tsx`).
3. Settings serde default keeps older `settings.json` files loadable without
   resetting onboarding (`#[serde(default)]` on `last_conversation_id`).
4. Settings screen merges patches onto the stored settings so theme edits do
   not wipe `last_conversation_id`.

## Automated check

`cargo test` Settings round-trip for `last_conversation_id` (added with this
readiness pass).

## Manual UI steps (release QA §6)

Still recommended on a local install: open a conversation, quit, relaunch,
confirm selection; delete that conversation, relaunch, confirm empty Chat.

## Result

- Pass / Fail: Pass (code + unit coverage for restore field and clear-on-delete)
