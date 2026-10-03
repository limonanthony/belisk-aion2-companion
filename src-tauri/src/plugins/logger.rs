use std::path::PathBuf;
use std::sync::Arc;

use aether_core::logger::AppLogger;
use tauri::{
    plugin::{Builder, TauriPlugin},
    AppHandle, Emitter, Manager, Runtime, State,
};

/// How many lines the log window backfills when it opens.
const BACKFILL_LINES: usize = 500;

fn build_logger<R: Runtime>(app: &AppHandle<R>) -> AppLogger {
    let app = app.clone();
    AppLogger::new(
        resolve_log_path(&app),
        Box::new(move |event| {
            let _ = app.emit("app-logger", event);
        }),
    )
}

fn resolve_log_path<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    if let Ok(dir) = app.path().app_log_dir() {
        return dir.join("aether.log");
    }
    if let Ok(dir) = app.path().app_data_dir() {
        return dir.join("logs").join("aether.log");
    }
    std::env::temp_dir().join("aether").join("aether.log")
}

#[tauri::command]
pub fn get_app_logger_debug_enabled(logger: State<'_, Arc<AppLogger>>) -> bool {
    logger.is_debug_enabled()
}

#[tauri::command]
pub fn set_app_logger_debug_enabled(logger: State<'_, Arc<AppLogger>>, enabled: bool) -> bool {
    logger.set_debug_enabled(enabled);
    logger.is_debug_enabled()
}

/// Backfill for the log window.
///
/// The window only ever listened for live `app-logger` events, so opening it
/// after startup -- which is when nearly everything is logged -- showed an empty
/// pane and read as a broken feature.
#[tauri::command]
pub fn read_app_log_tail(logger: State<'_, Arc<AppLogger>>) -> Vec<String> {
    logger.tail(BACKFILL_LINES)
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("logger")
        .setup(|app, _| {
            app.manage(Arc::new(build_logger(app.app_handle())));
            Ok(())
        })
        .build()
}
