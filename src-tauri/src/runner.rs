use crate::config::{self, MachineConfig, ProductConfig};
use crate::iss;
use crate::kits;
use crate::manifest;
use crate::probe;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::Instant;

/// The nine stages a run goes through. The order is the product's own model of the job.
pub const STAGES: [&str; 9] = ["预检", "清空", "拷 exe", "拷清单", "windeployqt", "排除", "生成 iss", "ISCC", "清理"];

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    Deploy,
    Package,
    Both,
}

impl Stage {
    pub fn deploys(self) -> bool {
        matches!(self, Stage::Deploy | Stage::Both)
    }
    pub fn packages(self) -> bool {
        matches!(self, Stage::Package | Stage::Both)
    }
}

pub trait ToolRunner: Send + Sync {
    fn run(&self, program: &Path, args: &[String], cwd: &Path) -> Result<Output>;
}

pub struct RealTools;

impl ToolRunner for RealTools {
    fn run(&self, program: &Path, args: &[String], cwd: &Path) -> Result<Output> {
        let mut cmd = std::process::Command::new(program);
        cmd.args(args).current_dir(cwd);
        // The GUI has no console; without this, console children pop their own window.
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        // windeployqt asks qmake for its Qt paths through PATH. Put the tool's own
        // folder first so another Qt on PATH cannot redirect it.
        if let Some(dir) = program.parent().filter(|dir| !dir.as_os_str().is_empty()) {
            let inherited = std::env::var_os("PATH").unwrap_or_default();
            let paths = std::iter::once(dir.to_path_buf()).chain(std::env::split_paths(&inherited));
            if let Ok(path) = std::env::join_paths(paths) {
                cmd.env("PATH", path);
            }
        }
        cmd.output()
            .with_context(|| format!("启动 {} 失败", program.display()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageRun {
    pub name: String,
    pub ms: u64,
    /// done | fail | skipped
    pub state: String,
}

pub struct RunCtx<'a> {
    pub machine: &'a MachineConfig,
    pub product: &'a ProductConfig,
    pub tools: &'a dyn ToolRunner,
    pub probe: fn(&Path) -> probe::ProbeInfo,
    pub log: &'a dyn Fn(&str),
    pub step: &'a dyn Fn(usize),
    /// false only in tests that inspect the staging dir afterwards
    pub cleanup: bool,
}

#[derive(Debug, Default)]
pub struct RunResult {
    pub ok: bool,
    pub message: String,
    pub staging_kept: String,
    pub installer: String,
    pub version: String,
    pub stages: Vec<StageRun>,
}

struct Ticker<'a> {
    step: &'a dyn Fn(usize),
    runs: Vec<StageRun>,
    current: Option<(usize, Instant)>,
}

impl<'a> Ticker<'a> {
    fn new(step: &'a dyn Fn(usize)) -> Self {
        let mut t = Ticker { step, runs: Vec::with_capacity(9), current: None };
        for name in STAGES {
            t.runs.push(StageRun { name: name.into(), ms: 0, state: "skipped".into() });
        }
        t
    }

    fn begin(&mut self, index: usize) {
        self.close("done");
        (self.step)(index);
        self.current = Some((index, Instant::now()));
    }

    fn close(&mut self, state: &str) {
        if let Some((index, at)) = self.current.take() {
            self.runs[index].ms = at.elapsed().as_millis() as u64;
            self.runs[index].state = state.into();
        }
    }

    fn finish(&mut self, ok: bool) -> Vec<StageRun> {
        self.close(if ok { "done" } else { "fail" });
        self.runs.clone()
    }
}

pub fn run_product(ctx: &RunCtx<'_>, stage: Stage) -> RunResult {
    let mut ticker = Ticker::new(ctx.step);
    match run_inner(ctx, stage, &mut ticker) {
        Ok(mut r) => {
            r.stages = ticker.finish(true);
            r
        }
        Err(e) => {
            let stages = ticker.finish(false);
            let staging_kept = manifest::staging_dir(ctx.machine, &ctx.product.app_name)
                .map(|dir| kept(&dir))
                .unwrap_or_default();
            RunResult {
                ok: false,
                message: format!("{e:#}"),
                staging_kept,
                stages,
                ..Default::default()
            }
        }
    }
}

fn kept(staging: &Path) -> String {
    if staging.exists() {
        staging.to_string_lossy().to_string()
    } else {
        String::new()
    }
}

fn run_inner(ctx: &RunCtx<'_>, stage: Stage, t: &mut Ticker<'_>) -> Result<RunResult> {
    let p = ctx.product;
    let m = ctx.machine;

    t.begin(0);
    config::validate_product(p)?;
    let info = (ctx.probe)(Path::new(&p.exe_path));
    if !info.ok {
        bail!("{}", info.error);
    }
    let kit = kits::resolve(&kits::scan(&m.qt_root), info.qt_major, &info.arch_bits.to_string(), &info.compiler, &p.kit)
        .map_err(|e| anyhow::anyhow!(e))?;
    if p.kit != "auto" && !p.kit.is_empty() {
        (ctx.log)(&format!("使用手动指定的套件 {}（未校验与 exe 是否匹配）", kit.id));
    } else {
        (ctx.log)(&format!("识别为 {} → {}", kit.id, kit.windeployqt));
    }
    let version_full = if p.version_override.trim().is_empty() {
        info.version_full.clone()
    } else {
        p.version_override.trim().to_string()
    };
    if version_full.is_empty() {
        bail!("exe 没有版本资源，且没设版本覆盖");
    }
    let version_short = probe::short_version(&version_full);

    let staging = manifest::staging_dir(m, &p.app_name).map_err(|e| anyhow::anyhow!(e))?;

    if stage.deploys() {
        deploy(ctx, t, &kit.windeployqt, &staging, Path::new(&p.exe_path), &info.compiler)?;
    }

    let mut installer = PathBuf::new();
    if stage.packages() {
        installer = package(ctx, t, &version_short, &staging)?;
    }

    if ctx.cleanup && stage.packages() {
        t.begin(8);
        let clean = manifest::clear_app_dirs(m, &p.app_name);
        if let Err(e) = &clean {
            (ctx.log)(&format!("警告：清理失败：{e}"));
        }
        return Ok(RunResult {
            ok: true,
            message: if clean.is_ok() { "完成，暂存已清理".into() } else { "完成，暂存未能清理".into() },
            staging_kept: kept(&staging),
            installer: installer.to_string_lossy().to_string(),
            version: version_short,
            stages: Vec::new(),
        });
    }

    Ok(RunResult {
        ok: true,
        message: "完成".into(),
        staging_kept: kept(&staging),
        installer: installer.to_string_lossy().to_string(),
        version: version_short,
        stages: Vec::new(),
    })
}

const MINGW_RUNTIME: [&str; 4] = [
    "libgcc_s_seh-1.dll",
    "libgcc_s_dw2-1.dll",
    "libstdc++-6.dll",
    "libwinpthread-1.dll",
];
const LLVM_RUNTIME: [&str; 2] = ["libc++.dll", "libunwind.dll"];

fn copy_runtime_dlls(windeployqt: &str, staging: &Path, compiler: &str) -> Result<()> {
    let names = if compiler == "llvm-mingw" { &LLVM_RUNTIME[..] } else { &MINGW_RUNTIME[..] };
    let Some(bin) = Path::new(windeployqt).parent() else {
        return Ok(());
    };
    for name in names {
        let src = bin.join(name);
        let dst = staging.join(name);
        if src.is_file() && !dst.exists() {
            std::fs::copy(&src, &dst).with_context(|| format!("复制运行库 {} 失败", src.display()))?;
        }
    }
    Ok(())
}

fn deploy(ctx: &RunCtx<'_>, t: &mut Ticker<'_>, windeployqt: &str, staging: &Path, exe_src: &Path, compiler: &str) -> Result<()> {
    let p = ctx.product;

    t.begin(1);
    manifest::clear_app_dirs(ctx.machine, &p.app_name).map_err(|e| anyhow::anyhow!(e))?;
    std::fs::create_dir_all(staging)?;

    t.begin(2);
    let exe_dst = staging.join(format!("{}.exe", p.app_name));
    std::fs::copy(exe_src, &exe_dst).with_context(|| format!("复制 {} 失败", exe_src.display()))?;

    t.begin(3);
    let exe_dir = exe_src.parent().unwrap_or(Path::new(".")).to_path_buf();
    manifest::apply_includes(p, &exe_dir, staging).map_err(|e| anyhow::anyhow!(e))?;

    t.begin(4);
    // Let windeployqt detect debug/release from the exe. Forcing --release also
    // rejects plugins that its MinGW check misreads as debug (separate_debug_info).
    // Its GCC lookup needs g++ on PATH, which we do not have; copy the runtime instead.
    let mut args = vec!["--no-compiler-runtime".to_string()];
    args.extend(p.windeployqt_args.iter().cloned());
    args.push(exe_dst.file_name().unwrap().to_string_lossy().to_string());
    (ctx.log)(&format!("{windeployqt} {}", args.join(" ")));
    let out = ctx.tools.run(Path::new(windeployqt), &args, staging)?;
    log_output(ctx, &out);
    if !out.status.success() {
        bail!("windeployqt 失败，退出码 {:?}", out.status.code());
    }
    copy_runtime_dlls(windeployqt, staging, compiler)?;

    t.begin(5);
    for w in manifest::apply_excludes(p, staging).map_err(|e| anyhow::anyhow!(e))? {
        (ctx.log)(&format!("警告：{w}"));
    }
    Ok(())
}

fn package(ctx: &RunCtx<'_>, t: &mut Ticker<'_>, version_short: &str, staging: &Path) -> Result<PathBuf> {
    let p = ctx.product;
    let m = ctx.machine;

    if !staging.join(format!("{}.exe", p.app_name)).exists() {
        t.begin(6);
        bail!("暂存目录 {} 里没有 {}，请先部署", staging.display(), p.app_name);
    }
    t.begin(6);
    let work = manifest::iss_dir(m, &p.app_name).map_err(|e| anyhow::anyhow!(e))?;
    std::fs::create_dir_all(&work)?;
    let script = iss::render(p, version_short, staging);
    std::fs::write(work.join("common_setup.iss"), iss::TEMPLATE)?;
    std::fs::write(work.join(format!("{}.iss", p.app_name)), &script)?;
    (ctx.log)(&format!("--- {}.iss ---\n{}", p.app_name, script));

    t.begin(7);
    let installer = Path::new(&p.output_dir).join(format!("{} {} Installer.exe", p.app_name, version_short));
    if let Some(dir) = installer.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let before = installer.metadata().and_then(|m| m.modified()).ok();
    let iss_path = work.join(format!("{}.iss", p.app_name));
    (ctx.log)(&format!("{} {}", m.iscc_exe, iss_path.display()));
    let out = ctx.tools.run(Path::new(&m.iscc_exe), &[iss_path.to_string_lossy().to_string()], &work)?;
    log_output(ctx, &out);
    if !out.status.success() {
        bail!("ISCC 失败，退出码 {:?}", out.status.code());
    }
    let after = installer.metadata().and_then(|m| m.modified()).ok();
    let fresh = match (before, after) {
        (Some(before), Some(after)) => after > before,
        (None, Some(_)) => true,
        _ => false,
    };
    if !fresh {
        bail!("ISCC 返回成功，但没找到新生成的 {}", installer.display());
    }
    Ok(installer)
}

fn log_output(ctx: &RunCtx<'_>, out: &Output) {
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        (ctx.log)(line);
    }
    for line in String::from_utf8_lossy(&out.stderr).lines() {
        (ctx.log)(&format!("[stderr] {line}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ManifestEntry;
    use std::os::windows::process::ExitStatusExt;

    struct FakeTools {
        installer: PathBuf,
        fail_iscc: bool,
    }

    fn ok_status() -> Output {
        Output { status: std::process::ExitStatus::from_raw(0), stdout: Vec::new(), stderr: Vec::new() }
    }

    impl ToolRunner for FakeTools {
        fn run(&self, program: &Path, _args: &[String], cwd: &Path) -> Result<Output> {
            let name = program.file_name().unwrap().to_string_lossy().to_lowercase();
            if name.contains("windeployqt") {
                std::fs::write(cwd.join("Qt5Core.dll"), b"x")?;
                std::fs::create_dir_all(cwd.join("plugins").join("sqldrivers"))?;
                std::fs::write(cwd.join("plugins").join("sqldrivers").join("qsqlite.dll"), b"x")?;
                return Ok(ok_status());
            }
            if self.fail_iscc {
                return Ok(Output { status: std::process::ExitStatus::from_raw(1), stdout: Vec::new(), stderr: Vec::new() });
            }
            std::fs::write(&self.installer, b"installer")?;
            Ok(ok_status())
        }
    }

    struct Env {
        _tmp: tempfile::TempDir,
        machine: MachineConfig,
        product: ProductConfig,
        installer: PathBuf,
    }

    fn setup() -> Env {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let build = root.join("build");
        std::fs::create_dir_all(&build).unwrap();
        std::fs::write(build.join("App.exe"), b"MZ").unwrap();
        std::fs::write(build.join("app.ini"), b"cfg").unwrap();
        let kit_bin = root.join("qt/6.9.0/mingw_64/bin");
        std::fs::create_dir_all(&kit_bin).unwrap();
        std::fs::write(kit_bin.join("windeployqt.exe"), b"x").unwrap();

        let machine = MachineConfig {
            qt_root: root.join("qt").to_string_lossy().to_string(),
            staging_root: root.join("staging").to_string_lossy().to_string(),
            iscc_exe: root.join("ISCC.exe").to_string_lossy().to_string(),
            ..Default::default()
        };
        let product = ProductConfig {
            app_name: "App".into(),
            app_id: "9D5A0C45-3B1E-4B7F-9C2A-8E6D1F0A2B33".into(),
            exe_path: build.join("App.exe").to_string_lossy().to_string(),
            output_dir: root.join("out").to_string_lossy().to_string(),
            kit: "auto".into(),
            ..Default::default()
        };
        let installer = root.join("out").join("App 1.2.3 Installer.exe");
        Env { _tmp: tmp, machine, product, installer }
    }

    fn fake_probe(_: &Path) -> probe::ProbeInfo {
        probe::ProbeInfo {
            ok: true,
            arch_bits: 64,
            qt_major: 6,
            compiler: "mingw".into(),
            version_full: "1.2.3.4".into(),
            ..Default::default()
        }
    }

    fn ctx<'a>(env: &'a Env, tools: &'a FakeTools, product: &'a ProductConfig, cleanup: bool) -> RunCtx<'a> {
        RunCtx {
            machine: &env.machine,
            product,
            tools,
            probe: fake_probe,
            log: &|_| {},
            step: &|_| {},
            cleanup,
        }
    }

    fn staging_of(env: &Env) -> PathBuf {
        manifest::staging_dir(&env.machine, "App").unwrap()
    }

    #[test]
    fn deploy_is_idempotent_and_drops_removed_files() {
        let env = setup();
        let tools = FakeTools { installer: env.installer.clone(), fail_iscc: false };
        let mut product = env.product.clone();
        product.include = vec![ManifestEntry { source: "app.ini".into(), target: "config/app.ini".into() }];
        product.exclude = vec!["plugins\\sqldrivers\\*".into()];

        deploy(&ctx(&env, &tools, &product, false), &mut Ticker::new(&|_| {}), "windeployqt.exe", &staging_of(&env), Path::new(&product.exe_path), "mingw").unwrap();
        let first = list(&staging_of(&env));
        assert!(has(&first, "config/app.ini"));
        assert!(has(&first, "App.exe"));
        assert!(!has(&first, "plugins/sqldrivers/qsqlite.dll"), "exclude must delete it");

        deploy(&ctx(&env, &tools, &product, false), &mut Ticker::new(&|_| {}), "windeployqt.exe", &staging_of(&env), Path::new(&product.exe_path), "mingw").unwrap();
        assert_eq!(first, list(&staging_of(&env)), "second run must produce the same tree");

        product.include.clear();
        product.exclude.clear();
        deploy(&ctx(&env, &tools, &product, false), &mut Ticker::new(&|_| {}), "windeployqt.exe", &staging_of(&env), Path::new(&product.exe_path), "mingw").unwrap();
        assert!(!has(&list(&staging_of(&env)), "config/app.ini"), "stale include must be gone");
    }

    #[test]
    fn package_writes_iss_files() {
        let env = setup();
        let tools = FakeTools { installer: env.installer.clone(), fail_iscc: false };
        let product = env.product.clone();
        let c = ctx(&env, &tools, &product, true);
        let mut t = Ticker::new(&|_| {});

        deploy(&c, &mut t, "windeployqt.exe", &staging_of(&env), Path::new(&product.exe_path), "mingw").unwrap();
        let installer = package(&c, &mut t, "1.2.3", &staging_of(&env)).unwrap();
        assert!(installer.exists());

        let work = manifest::iss_dir(&env.machine, "App").unwrap();
        let script = std::fs::read_to_string(work.join("App.iss")).unwrap();
        assert!(script.contains("#define AppName \"App\""));
        assert!(script.contains("#define AppVersion \"1.2.3\""));
        assert!(script.contains(&format!("#define SourceDir \"{}\"", staging_of(&env).display())));
        assert!(script.ends_with("#include \"common_setup.iss\"\n"));
        assert!(!work.join("local.iss").exists(), "local.iss is gone from the contract");
        assert!(!work.join(".deploy.iss").exists(), ".deploy.iss is gone from the contract");

        assert_eq!(std::fs::read_to_string(work.join("common_setup.iss")).unwrap(), iss::TEMPLATE);

        manifest::remove_dir(&staging_of(&env)).unwrap();
        manifest::remove_dir(&work).unwrap();
        assert!(!staging_of(&env).exists());
    }

    /// The installer name QDeploy looks for and AppVersion in the script must come from one value.
    #[test]
    fn version_override_reaches_script_and_filename() {
        let env = setup();
        let mut product = env.product.clone();
        product.version_override = " 2.3.4.5 ".into();
        let installer = env.installer.parent().unwrap().join("App 2.3.4 Installer.exe");
        let tools = FakeTools { installer: installer.clone(), fail_iscc: false };
        let r = run_product(&ctx(&env, &tools, &product, false), Stage::Both);
        assert!(r.ok, "{}", r.message);
        assert_eq!(Path::new(&r.installer), installer);

        let work = manifest::iss_dir(&env.machine, "App").unwrap();
        let script = std::fs::read_to_string(work.join("App.iss")).unwrap();
        assert!(script.contains("#define AppVersion \"2.3.4\""), "{script}");
    }

    #[test]
    fn failed_iscc_keeps_staging() {
        let env = setup();
        let tools = FakeTools { installer: env.installer.clone(), fail_iscc: true };
        let product = env.product.clone();
        let c = ctx(&env, &tools, &product, false);
        let mut t = Ticker::new(&|_| {});
        deploy(&c, &mut t, "windeployqt.exe", &staging_of(&env), Path::new(&product.exe_path), "mingw").unwrap();
        assert!(package(&c, &mut t, "1.2.3", &staging_of(&env)).is_err());
        assert!(staging_of(&env).exists(), "failure must leave evidence");
    }

    #[test]
    fn ticker_marks_the_failed_stage() {
        let step = |_: usize| {};
        let mut t = Ticker::new(&step);
        t.begin(0);
        t.begin(1);
        let stages = t.finish(false);
        assert_eq!(stages.len(), 9);
        assert_eq!(stages[0].state, "done");
        assert_eq!(stages[1].state, "fail");
        assert_eq!(stages[2].state, "skipped");
    }

    #[test]
    fn deploy_only_reports_kept_staging() {
        let env = setup();
        let tools = FakeTools { installer: env.installer.clone(), fail_iscc: false };
        let product = env.product.clone();
        let r = run_product(&ctx(&env, &tools, &product, true), Stage::Deploy);
        assert!(r.ok, "{}", r.message);
        assert_eq!(r.staging_kept, staging_of(&env).to_string_lossy().to_string());
        assert!(staging_of(&env).exists());
    }

    #[test]
    fn package_cleanup_reports_installer_and_no_staging() {
        let env = setup();
        let tools = FakeTools { installer: env.installer.clone(), fail_iscc: false };
        let product = env.product.clone();
        let r = run_product(&ctx(&env, &tools, &product, true), Stage::Both);
        assert!(r.ok, "{}", r.message);
        assert!(env.installer.exists());
        assert!(r.staging_kept.is_empty());
        assert!(!staging_of(&env).exists());
    }

    #[test]
    fn failed_run_reports_kept_staging() {
        let env = setup();
        let tools = FakeTools { installer: env.installer.clone(), fail_iscc: true };
        let product = env.product.clone();
        let r = run_product(&ctx(&env, &tools, &product, false), Stage::Both);
        assert!(!r.ok);
        assert_eq!(r.staging_kept, staging_of(&env).to_string_lossy().to_string());
        assert!(staging_of(&env).exists());
    }

    #[test]
    fn failed_rebuild_keeps_previous_installer() {
        let env = setup();
        std::fs::create_dir_all(env.installer.parent().unwrap()).unwrap();
        std::fs::write(&env.installer, b"old").unwrap();
        let tools = FakeTools { installer: env.installer.clone(), fail_iscc: true };
        let product = env.product.clone();
        let c = ctx(&env, &tools, &product, false);
        let mut t = Ticker::new(&|_| {});
        deploy(&c, &mut t, "windeployqt.exe", &staging_of(&env), Path::new(&product.exe_path), "mingw").unwrap();
        assert!(package(&c, &mut t, "1.2.3", &staging_of(&env)).is_err());
        assert_eq!(std::fs::read(&env.installer).unwrap(), b"old");
    }

    #[test]
    fn prepends_tool_dir_to_path() {
        let tools = RealTools;
        let cmd = Path::new("C:\\Windows\\System32\\cmd.exe");
        let out = tools.run(cmd, &["/c".into(), "echo %PATH%".into()], Path::new(".")).unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(
            text.trim_start().to_lowercase().starts_with("c:\\windows\\system32;"),
            "child PATH was {text}"
        );
    }

    #[test]
    fn copies_missing_mingw_runtime() {
        let env = setup();
        let kit_bin = Path::new(&env.machine.qt_root).join("6.9.0/mingw_64/bin");
        let names = ["libgcc_s_seh-1.dll", "libstdc++-6.dll", "libwinpthread-1.dll"];
        for name in names {
            std::fs::write(kit_bin.join(name), b"rt").unwrap();
        }
        let tools = FakeTools { installer: env.installer.clone(), fail_iscc: false };
        let product = env.product.clone();
        let windeployqt = kit_bin.join("windeployqt.exe");
        let c = ctx(&env, &tools, &product, false);
        let mut t = Ticker::new(&|_| {});
        deploy(
            &c,
            &mut t,
            windeployqt.to_str().unwrap(),
            &staging_of(&env),
            Path::new(&product.exe_path),
            "mingw",
        )
        .unwrap();
        for name in names {
            assert!(staging_of(&env).join(name).is_file(), "{name} missing");
        }
    }

    fn has(files: &[String], name: &str) -> bool {
        files.iter().any(|f| f == name)
    }

    fn list(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().strip_prefix(dir).unwrap().to_string_lossy().replace('\\', "/"))
            .collect();
        v.sort();
        v
    }
}
