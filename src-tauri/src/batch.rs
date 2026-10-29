use crate::config::Config;
use crate::history::{self, RunRecord};
use crate::runner::{self, RealTools, RunCtx, Stage};
use serde::Serialize;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductEvent {
    app_name: String,
    status: &'static str,
    /// stage index while running; -1 when it is the final verdict
    stage: i64,
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
    cancelled: usize,
}

fn emit_product(
    app: &AppHandle,
    name: &str,
    status: &'static str,
    stage: i64,
    message: &str,
    staging_kept: &str,
    installer: &str,
) {
    let _ = app.emit(
        "run://product",
        ProductEvent {
            app_name: name.to_string(),
            status,
            stage,
            message: message.to_string(),
            staging_kept: staging_kept.to_string(),
            installer: installer.to_string(),
        },
    );
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
        let _ = self.app.emit("run://log", LogEvent { app_name: self.name.clone(), line: line.to_string() });
    }

    fn event(&self, status: &'static str, stage: i64, message: &str, staging_kept: &str, installer: &str) {
        emit_product(&self.app, &self.name, status, stage, message, staging_kept, installer);
    }
}

fn log_root() -> std::path::PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    std::path::PathBuf::from(base).join("QDeploy").join("logs").join(stamp.to_string())
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn start(app: AppHandle, cfg: Config, names: Vec<String>, stage: Stage, running: Arc<AtomicBool>, cancel: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let dir = log_root();
        let _ = std::fs::create_dir_all(&dir);
        let mut ok = 0usize;
        let mut failed = 0usize;
        let mut started = 0usize;

        for (i, name) in names.iter().enumerate() {
            if cancel.load(Ordering::SeqCst) {
                break;
            }
            started = i + 1;
            let Some(product) = cfg.products.iter().find(|p| p.app_name == *name).cloned() else {
                failed += 1;
                let _ = app.emit("run://log", LogEvent { app_name: name.clone(), line: "配置里没有这个产品".into() });
                emit_product(&app, name, "failed", -1, "配置里没有这个产品", "", "");
                continue;
            };
            let Ok(file) = std::fs::File::create(dir.join(format!("{name}.log"))) else {
                failed += 1;
                emit_product(&app, name, "failed", -1, "无法创建日志文件", "", "");
                continue;
            };
            let reporter = Reporter { app: app.clone(), name: name.clone(), file: Mutex::new(file) };
            reporter.event("running", -1, "", "", "");
            reporter.log(&format!("=== {name} / {stage:?} ==="));

            let run_started = Instant::now();
            let tools = RealTools;
            let log = |l: &str| reporter.log(l);
            let step = |i: usize| reporter.event("running", i as i64, "", "", "");
            let ctx = RunCtx {
                machine: &cfg.machine,
                product: &product,
                tools: &tools,
                probe: crate::probe::probe,
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

            history::append(RunRecord {
                app_name: name.clone(),
                at: now(),
                duration_ms: run_started.elapsed().as_millis() as u64,
                ok: r.ok,
                message: r.message.clone(),
                installer: r.installer.clone(),
                version: r.version.clone(),
                staging_kept: r.staging_kept.clone(),
                stages: r.stages.clone(),
            });

            reporter.event(if r.ok { "ok" } else { "failed" }, -1, &r.message, &r.staging_kept, &r.installer);
        }

        let cancelled = if cancel.load(Ordering::SeqCst) {
            for name in &names[started..] {
                emit_product(&app, name, "failed", -1, "已取消，未执行", "", "");
            }
            names.len().saturating_sub(started)
        } else {
            0
        };
        running.store(false, Ordering::SeqCst);
        let _ = app.emit("run://done", DoneEvent { ok, failed, cancelled });
    });
}
