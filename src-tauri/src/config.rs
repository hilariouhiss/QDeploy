use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MachineConfig {
    pub qt_root: String,
    pub iscc_exe: String,
    pub staging_root: String,
    pub iss_template_path: String,
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

pub fn load() -> Config {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
            eprintln!("config.json unreadable ({e}), starting empty: {}", path.display());
            Config::default()
        }),
        Err(_) => Config::default(),
    }
}

pub fn save(cfg: &Config) -> Result<()> {
    let path = config_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(cfg)?)?;
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
}
