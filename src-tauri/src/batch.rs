use crate::config::Config;
use crate::runner::{self, RealTools, RunCtx, Stage};
use serde::Serialize;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductEvent {
    app_name: String,
    status: &'static str,
    step: String,
    message: String,
    staging_kept: String,
    installer: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LogEvent {
    app_name: String,
    line: String,
}

#[derive(Clone, Serialize)]
struct DoneEvent {
    ok: usize,
    failed: usize,
}

struct Reporter {
    app: AppHandle,
    name: String,
    file: Mutex<std::fs::File>,
}

impl Reporter {
    fn log(&self, line: &str) {
        if let Ok(mut f) = self.file.lock() {
            let _ = writeln!(f, "{line}");
        }
        let _ = self.app.emit(
            "run://log",
            LogEvent { app_name: self.name.clone(), line: line.to_string() },
        );
    }

    fn event(&self, status: &'static str, step: &str, message: &str, staging_kept: &str, installer: &str) {
        let _ = self.app.emit(
            "run://product",
            ProductEvent {
                app_name: self.name.clone(),
                status,
                step: step.to_string(),
                message: message.to_string(),
                staging_kept: staging_kept.to_string(),
                installer: installer.to_string(),
            },
        );
    }
}

fn log_root() -> std::path::PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    std::path::PathBuf::from(base).join("QDeploy").join("logs").join(stamp.to_string())
}

pub fn start(app: AppHandle, cfg: Config, names: Vec<String>, stage: Stage, running: Arc<AtomicBool>, cancel: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let dir = log_root();
        let _ = std::fs::create_dir_all(&dir);
        let mut ok = 0usize;
        let mut failed = 0usize;

        for name in names {
            if cancel.load(Ordering::SeqCst) {
                break;
            }
            let Some(product) = cfg.products.iter().find(|p| p.app_name == name).cloned() else {
                failed += 1;
                let _ = app.emit("run://log", LogEvent { app_name: name.clone(), line: "配置里没有这个产品".into() });
                continue;
            };
            let Ok(file) = std::fs::File::create(dir.join(format!("{name}.log"))) else {
                failed += 1;
                continue;
            };
            let reporter = Reporter { app: app.clone(), name: name.clone(), file: Mutex::new(file) };
            reporter.event("running", "", "开始", "", "");
            reporter.log(&format!("=== {name} / {stage:?} ==="));

            let tools = RealTools;
            let log = |l: &str| reporter.log(l);
            let step = |s: &str| reporter.event("running", s, "", "", "");
            let ctx = RunCtx {
                machine: &cfg.machine,
                product: &product,
                tools: &tools,
                log: &log,
                step: &step,
                cleanup: true,
            };
            let r = runner::run_product(&ctx, stage);
            if r.ok {
                ok += 1;
                reporter.log(&format!("OK {}", r.installer));
            } else {
                failed += 1;
                reporter.log(&format!("FAILED {}", r.message));
            }
            reporter.event(
                if r.ok { "ok" } else { "failed" },
                "",
                &r.message,
                &r.staging_kept,
                &r.installer,
            );
        }

        running.store(false, Ordering::SeqCst);
        let _ = app.emit("run://done", DoneEvent { ok, failed });
    });
}
