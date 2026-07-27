mod chat_proxy;
mod commands;
mod config;
mod diagnostics;
pub mod errors;
pub mod gguf;
mod logging;
mod paths;
mod process;
pub mod runtime;
pub mod storage;
pub mod types;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    paths::ensure_dirs().expect("failed to create Omnira data directories");
    logging::info("app.start", env!("CARGO_PKG_VERSION"));

    let storage = storage::Storage::open().expect("failed to open local database");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            storage,
            runtime: runtime::RuntimeManager::default(),
            chat_streams: chat_proxy::ChatStreams::default(),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::list_models,
            commands::add_model,
            commands::remove_model,
            commands::rename_model,
            commands::list_conversations,
            commands::create_conversation,
            commands::rename_conversation,
            commands::set_conversation_model,
            commands::delete_conversation,
            commands::clear_conversations,
            commands::list_messages,
            commands::add_message,
            commands::runtime_status,
            commands::chat_endpoint,
            commands::start_runtime,
            commands::stop_runtime,
            commands::chat_stream,
            commands::chat_cancel,
            commands::diagnostics_snapshot,
            commands::diagnostics_export,
        ])
        .on_window_event(|window, event| {
            // Shut down the managed runtime on app exit (normal path; the Job
            // Object covers crash/force-kill).
            if let tauri::WindowEvent::Destroyed = event {
                use tauri::Manager;
                let state: tauri::State<AppState> = window.state();
                if let Some(rt) = state.runtime.take_runtime() {
                    tauri::async_runtime::block_on(runtime::stop(rt));
                }
                logging::info("app.exit", "");
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
