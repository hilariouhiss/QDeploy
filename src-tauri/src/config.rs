use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MachineConfig {
    pub qt_root: String,
    pub iscc_exe: String,
    pub staging_root: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ManifestEntry {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProductConfig {
    pub app_name: String,
    pub app_id: String,
    pub exe_path: String,
    pub kit: String,
    pub output_dir: String,
    pub windeployqt_args: Vec<String>,
    pub version_override: String,
    pub include: Vec<ManifestEntry>,
    pub exclude: Vec<String>,
}

impl Default for ProductConfig {
    fn default() -> Self {
        ProductConfig {
            app_name: String::new(),
            app_id: String::new(),
            exe_path: String::new(),
            kit: "auto".into(),
            output_dir: String::new(),
            windeployqt_args: Vec::new(),
            version_override: String::new(),
            include: Vec::new(),
            exclude: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub schema_version: u32,
    pub machine: MachineConfig,
    pub products: Vec<ProductConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            schema_version: 1,
            machine: MachineConfig::default(),
            products: Vec::new(),
        }
    }
}

pub fn config_path() -> PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    PathBuf::from(base).join("QDeploy").join("config.json")
}

pub fn load() -> (Config, Option<String>) {
    load_file(&config_path())
}

/// A parse error must never silently overwrite the user's only copy: keep a backup and tell the UI.
fn load_file(path: &Path) -> (Config, Option<String>) {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => return (Config::default(), None),
    };
    match serde_json::from_str(&text) {
        Ok(cfg) => (cfg, None),
        Err(e) => {
            let backup = path.with_file_name(format!("config.broken-{}.json", epoch_secs()));
            let warning = match std::fs::copy(path, &backup) {
                Ok(_) => format!(
                    "config.json 解析失败（{e}），已备份到 {}，本次从空配置开始",
                    backup.display()
                ),
                Err(backup_err) => format!(
                    "config.json 解析失败（{e}），备份也失败（{backup_err}），本次从空配置开始"
                ),
            };
            (Config::default(), Some(warning))
        }
    }
}

fn epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn save(cfg: &Config) -> Result<()> {
    save_file(&config_path(), cfg)
}

fn save_file(path: &Path, cfg: &Config) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    // Write then rename so a crash cannot leave a half-written config.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(cfg)?)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

const ILLEGAL: [char; 9] = ['\\', '/', ':', '*', '?', '"', '<', '>', '|'];

pub fn validate_app_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        bail!("应用名不能为空");
    }
    if name.chars().any(|c| ILLEGAL.contains(&c)) {
        bail!("应用名不能包含 \\ / : * ? \" < > | 这些字符");
    }
    if name.ends_with(' ') || name.ends_with('.') {
        bail!("应用名不能以空格或点结尾");
    }
    if name == "." || name == ".." || name == ".iss" {
        bail!("应用名不能是 . / .. / .iss");
    }
    Ok(())
}

pub fn validate_app_id(id: &str) -> Result<()> {
    let ok = id.len() == 36
        && id.chars().enumerate().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => c == '-',
            _ => c.is_ascii_hexdigit(),
        });
    if !ok {
        bail!("GUID 格式不对：应为 36 位、带连字符、不带花括号");
    }
    Ok(())
}

/// Checks the pieces the Inno Setup template depends on.
pub fn validate_product(p: &ProductConfig) -> Result<()> {
    validate_app_name(&p.app_name)?;
    validate_app_id(&p.app_id)?;
    if p.exe_path.trim().is_empty() {
        bail!("exe 路径不能为空");
    }
    let exe = PathBuf::from(&p.exe_path);
    let want = format!("{}.exe", p.app_name);
    let got = exe.file_name().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
    if got != want.to_lowercase() {
        bail!("exe 文件名必须叫 {want}（模板按 AppName 拼 exe 名），当前是 {got}");
    }
    if p.output_dir.trim().is_empty() {
        bail!("安装包输出目录不能为空");
    }
    Ok(())
}

/// Names become folder and file names, so they must be legal and unique before anything is written.
pub fn validate_names(products: &[ProductConfig]) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for p in products {
        validate_app_name(&p.app_name)?;
        let key = p.app_name.to_lowercase();
        if !seen.insert(key) {
            bail!("应用名重复（Windows 路径不区分大小写）：{}", p.app_name);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let mut cfg = Config::default();
        cfg.machine.qt_root = "C:\\Qt".into();
        let mut p = ProductConfig::default();
        p.app_name = "AlphaTool".into();
        p.app_id = "9D5A0C45-3B1E-4B7F-9C2A-8E6D1F0A2B33".into();
        p.exe_path = "D:\\b\\AlphaTool.exe".into();
        p.output_dir = "D:\\out".into();
        cfg.products.push(p);
        let text = serde_json::to_string(&cfg).unwrap();
        let back: Config = serde_json::from_str(&text).unwrap();
        assert_eq!(back.products[0].app_name, "AlphaTool");
        assert_eq!(back.products[0].kit, "auto");
    }

    #[test]
    fn missing_fields_default() {
        let cfg: Config = serde_json::from_str(r#"{"products":[{"appName":"A"}]}"#).unwrap();
        assert_eq!(cfg.products[0].kit, "auto");
        assert!(cfg.products[0].include.is_empty());
        assert_eq!(cfg.schema_version, 1);
    }

    #[test]
    fn guards_names() {
        assert!(validate_app_name("AlphaTool").is_ok());
        assert!(validate_app_name("Alpha Tool").is_ok());
        assert!(validate_app_name("").is_err());
        assert!(validate_app_name("../evil").is_err());
        assert!(validate_app_name("a:b").is_err());
        assert!(validate_app_name("trail.").is_err());
    }

    #[test]
    fn guards_guid() {
        assert!(validate_app_id("9D5A0C45-3B1E-4B7F-9C2A-8E6D1F0A2B33").is_ok());
        assert!(validate_app_id("{9D5A0C45-3B1E-4B7F-9C2A-8E6D1F0A2B33}").is_err());
        assert!(validate_app_id("nope").is_err());
    }

    #[test]
    fn corrupt_config_is_backed_up() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config.json");
        std::fs::write(&path, b"{ not json").unwrap();
        let (cfg, warning) = load_file(&path);
        assert!(cfg.products.is_empty());
        let warning = warning.expect("a warning");
        assert!(warning.contains("config.broken"), "{warning}");
        let backup = std::fs::read_dir(tmp.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .find(|e| e.file_name().to_string_lossy().starts_with("config.broken-"))
            .expect("backup");
        assert_eq!(std::fs::read(backup.path()).unwrap(), b"{ not json");
    }

    #[test]
    fn save_file_replaces_existing() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nested").join("config.json");
        let mut cfg = Config::default();
        cfg.machine.qt_root = "C:\\Qt".into();
        save_file(&path, &cfg).unwrap();
        cfg.machine.qt_root = "D:\\Qt".into();
        save_file(&path, &cfg).unwrap();
        let (back, warning) = load_file(&path);
        assert!(warning.is_none());
        assert_eq!(back.machine.qt_root, "D:\\Qt");
    }

    #[test]
    fn rejects_duplicate_names_case_insensitively() {
        let mut a = ProductConfig::default();
        a.app_name = "App".into();
        let mut b = a.clone();
        b.app_name = "app".into();
        assert!(validate_names(&[a.clone()]).is_ok());
        assert!(validate_names(&[a, b]).is_err());
    }
}
