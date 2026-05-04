//! Session telemetry persistence — single rolling JSON file.
//!
//! The frontend collects user interactions, IPC calls, errors, and
//! lifecycle events into an in-memory queue and periodically flushes
//! the *entire* queue to disk by calling `telemetry_persist`. We
//! overwrite the file on every flush; "last session only" falls out
//! naturally because the in-memory queue starts empty on each app
//! boot and the first flush replaces whatever the previous run wrote.
//!
//! Path resolution uses Tauri's platform-aware `app_log_dir()`:
//!   - macOS: `~/Library/Logs/com.ataca.aurix/last-session.json`
//!   - Linux: `$XDG_DATA_HOME/com.ataca.aurix/logs/last-session.json`
//!   - Windows: `%LOCALAPPDATA%\com.ataca.aurix\logs\last-session.json`
//! `$AURIX_TELEMETRY_PATH` overrides for tests / one-off captures.

#![allow(dead_code)] // rust-analyzer can't see commands behind tauri::generate_handler!

use std::fs;
use std::path::PathBuf;

use serde::Serialize;
use tauri::Manager;

use super::lp::CommandError;

fn telemetry_path(app: &tauri::AppHandle) -> Result<PathBuf, CommandError> {
    if let Ok(p) = std::env::var("AURIX_TELEMETRY_PATH") {
        let path = PathBuf::from(p);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        return Ok(path);
    }
    let log_dir = app.path().app_log_dir().map_err(|e| CommandError {
        message: format!("telemetry path resolution failed: {e}"),
        key_required: None,
    })?;
    fs::create_dir_all(&log_dir).map_err(|e| CommandError {
        message: format!("telemetry log dir create failed: {e}"),
        key_required: None,
    })?;
    Ok(log_dir.join("last-session.json"))
}

/// Returns the absolute path of the current session's telemetry file.
/// Useful for the user / agent to grep / cat after the fact.
#[tauri::command]
pub async fn telemetry_log_path(app: tauri::AppHandle) -> Result<String, CommandError> {
    let path = telemetry_path(&app)?;
    Ok(path.to_string_lossy().into_owned())
}

/// Overwrites the telemetry file with the supplied JSON blob. The
/// frontend assembles the entire current event queue + a small metadata
/// header and hands the whole thing over verbatim — no backend-side
/// schema knowledge.
#[tauri::command]
pub async fn telemetry_persist(
    app: tauri::AppHandle,
    json: String,
) -> Result<TelemetryWriteReport, CommandError> {
    let path = telemetry_path(&app)?;
    let bytes = json.len();
    fs::write(&path, json).map_err(|e| CommandError {
        message: format!("telemetry write failed: {e}"),
        key_required: None,
    })?;
    Ok(TelemetryWriteReport {
        path: path.to_string_lossy().into_owned(),
        bytes,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryWriteReport {
    pub path: String,
    pub bytes: usize,
}
