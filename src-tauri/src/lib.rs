mod batch;
mod config;
mod history;
mod iss;
mod kits;
mod manifest;
mod probe;
mod runner;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, State};

pub struct AppState {
    config: Mutex<config::Config>,
    load_warning: Mutex<Option<String>>,
    running: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
}

#[tauri::command]
fn load_config(state: State<'_, AppState>) -> config::Config {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn save_config(state: State<'_, AppState>, cfg: config::Config) -> Result<(), String> {
    config::validate_names(&cfg.products).map_err(|e| e.to_string())?;
    // 保存时就挡住太浅的工作目录，别等跑起来才发现；留空按默认位置校验。
    manifest::deploy_root(&cfg.machine)?;
    config::save(&cfg).map_err(|e| e.to_string())?;
    *state.config.lock().unwrap() = cfg;
    Ok(())
}

/// 工作目录留空时界面显示的默认位置（exe 旁的 QDeployData）。
#[tauri::command]
fn default_work_root() -> String {
    config::default_work_root()
}

/// 安装包目录（<工作目录>\installers）：产品页没跑过也能直接打开。
#[tauri::command]
fn installer_dir(state: State<'_, AppState>) -> Result<String, String> {
    let machine = state.config.lock().unwrap().machine.clone();
    manifest::installer_dir(&machine).map(|dir| dir.to_string_lossy().to_string())
}

/// 脚本页：内置模板，或按传入配置生成的产品 iss（不落盘）。
#[tauri::command]
fn iss_script(cfg: config::Config, app_name: Option<String>) -> Result<iss::IssScript, String> {
    iss::script_for(&cfg, app_name.as_deref())
}

#[tauri::command]
fn take_load_warning(state: State<'_, AppState>) -> Option<String> {
    state.load_warning.lock().unwrap().take()
}

#[tauri::command]
fn load_history() -> Vec<history::RunRecord> {
    history::load()
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
    let mut seen = std::collections::HashSet::new();
    let app_names: Vec<String> = app_names.into_iter().filter(|n| seen.insert(n.clone())).collect();
    for name in &app_names {
        config::validate_app_name(name).map_err(|e| e.to_string())?;
    }
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
    if state.running.load(Ordering::SeqCst) {
        return vec!["正在运行，先停止再清空".into()];
    }
    let cfg = state.config.lock().unwrap().clone();
    let mut messages = Vec::new();
    for name in app_names {
        messages.push(match manifest::clear_app_dirs(&cfg.machine, &name) {
            Ok(()) => format!("已清理 {name}"),
            Err(e) => e,
        });
    }
    messages
}

/// Explorer 只认目录：给文件就打开它所在的那层；文件已被删/改名时同样退到那层。
fn dir_to_open(path: &str) -> Result<std::path::PathBuf, String> {
    let p = std::path::Path::new(path.trim());
    if p.is_dir() {
        return Ok(p.to_path_buf());
    }
    if let Some(dir) = p.parent().filter(|d| d.is_dir()) {
        return Ok(dir.to_path_buf());
    }
    Err(format!("目录不存在：{path}"))
}

#[tauri::command]
fn open_dir(path: String) -> Result<(), String> {
    let dir = dir_to_open(&path)?;
    // explorer 是系统自带的，不为一次打开引入插件依赖。
    std::process::Command::new("explorer")
        .arg(&dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("打开 {} 失败：{e}", dir.display()))
}

pub fn run() {
    let (cfg, warning) = config::load();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            config: Mutex::new(cfg),
            load_warning: Mutex::new(warning),
            running: Arc::new(AtomicBool::new(false)),
            cancel: Arc::new(AtomicBool::new(false)),
        })
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            default_work_root,
            installer_dir,
            iss_script,
            take_load_warning,
            load_history,
            scan_kits,
            probe_exe,
            resolve_kit,
            start_run,
            cancel_run,
            is_running,
            clear_staging,
            open_dir
        ])
        .run(tauri::generate_context!())
        .expect("QDeploy failed to start");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dir_to_open_accepts_dirs_and_files_but_not_missing_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("App 1.2.3 Installer.exe");
        std::fs::write(&file, b"x").unwrap();

        assert_eq!(dir_to_open(tmp.path().to_str().unwrap()).unwrap(), tmp.path());
        assert_eq!(dir_to_open(file.to_str().unwrap()).unwrap(), tmp.path());
        // 安装包被删掉后仍应打开它所在的目录
        std::fs::remove_file(&file).unwrap();
        assert_eq!(dir_to_open(file.to_str().unwrap()).unwrap(), tmp.path());

        assert!(dir_to_open("").is_err());
        assert!(dir_to_open(&tmp.path().join("gone/deeper/x").to_string_lossy()).is_err());
    }
}
