mod batch;
mod config;
mod iss;
mod kits;
mod manifest;
mod probe;
mod runner;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, State};

pub struct AppState {
    config: Mutex<config::Config>,
    running: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
}

#[tauri::command]
fn load_config(state: State<'_, AppState>) -> config::Config {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn save_config(state: State<'_, AppState>, cfg: config::Config) -> Result<(), String> {
    config::save(&cfg).map_err(|e| e.to_string())?;
    *state.config.lock().unwrap() = cfg;
    Ok(())
}

#[tauri::command]
fn scan_kits(qt_root: String) -> Vec<kits::Kit> {
    kits::scan(&qt_root)
}

#[tauri::command]
fn probe_exe(path: String) -> probe::ProbeInfo {
    probe::probe(std::path::Path::new(&path))
}

#[tauri::command]
fn resolve_kit(exe_path: String, kit: String, qt_root: String) -> Result<kits::Kit, String> {
    let info = probe::probe(std::path::Path::new(&exe_path));
    if !info.ok {
        return Err(info.error);
    }
    kits::resolve(&kits::scan(&qt_root), info.qt_major, &info.arch_bits.to_string(), &info.compiler, &kit)
}

#[tauri::command]
fn start_run(app: AppHandle, state: State<'_, AppState>, app_names: Vec<String>, stage: runner::Stage) -> Result<(), String> {
    if state.running.swap(true, Ordering::SeqCst) {
        return Err("已有任务在跑，等它结束或先取消".into());
    }
    state.cancel.store(false, Ordering::SeqCst);
    let cfg = state.config.lock().unwrap().clone();
    batch::start(app, cfg, app_names, stage, state.running.clone(), state.cancel.clone());
    Ok(())
}

#[tauri::command]
fn cancel_run(state: State<'_, AppState>) {
    state.cancel.store(true, Ordering::SeqCst);
}

#[tauri::command]
fn is_running(state: State<'_, AppState>) -> bool {
    state.running.load(Ordering::SeqCst)
}

#[tauri::command]
fn clear_staging(state: State<'_, AppState>, app_names: Vec<String>) -> Vec<String> {
    let cfg = state.config.lock().unwrap().clone();
    let mut messages = Vec::new();
    for name in app_names {
        let result = manifest::staging_dir(&cfg.machine, &name).and_then(|d| manifest::remove_dir(&d));
        messages.push(match result {
            Ok(()) => format!("已清理 {name}"),
            Err(e) => e,
        });
    }
    messages
}

pub fn run() {
    let cfg = config::load();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            config: Mutex::new(cfg),
            running: Arc::new(AtomicBool::new(false)),
            cancel: Arc::new(AtomicBool::new(false)),
        })
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            scan_kits,
            probe_exe,
            resolve_kit,
            start_run,
            cancel_run,
            is_running,
            clear_staging
        ])
        .run(tauri::generate_context!())
        .expect("QDeploy failed to start");
}
