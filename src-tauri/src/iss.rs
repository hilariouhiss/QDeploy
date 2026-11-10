use crate::config::{validate_product, Config, ProductConfig};
use crate::manifest;
use crate::probe::{self, ProbeInfo};
use serde::Serialize;
use std::path::Path;

/// The installer template ships inside the exe: no file to point at, nothing to misconfigure.
pub const TEMPLATE: &str = include_str!("../templates/common_setup.iss");

fn q(s: &str) -> String {
    format!("\"{}\"", s)
}

/// The scripts page reads this: the built-in template, or one product's generated script.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssScript {
    pub app_name: String,
    pub title: String,
    pub text: String,
    /// Where packing writes the script; empty for the template.
    pub path: String,
}

/// The one script QDeploy hands to ISPP: every value the template needs, worked out elsewhere.
pub fn render(p: &ProductConfig, version: &str, source_dir: &Path, output_dir: &Path) -> String {
    let mut script = format!(
        "#define AppName {}\n#define AppGuid {}\n#define AppVersion {}\n#define SourceDir {}\n#define OutputDir {}\n",
        q(&p.app_name),
        q(&p.app_id),
        q(version),
        q(&source_dir.to_string_lossy()),
        q(&output_dir.to_string_lossy())
    );
    script.push_str("#include \"common_setup.iss\"\n");
    script
}

/// The version override wins over the exe's version resource; its short form names the installer.
pub fn version_short(p: &ProductConfig, info: &ProbeInfo) -> Result<String, String> {
    let full = if p.version_override.trim().is_empty() {
        info.version_full.clone()
    } else {
        p.version_override.trim().to_string()
    };
    if full.is_empty() {
        return Err("exe 没有版本资源，且没设版本覆盖".into());
    }
    Ok(probe::short_version(&full))
}

/// What the next pack would write, so the view never depends on a file cleanup already removed.
pub fn script_for(cfg: &Config, app_name: Option<&str>) -> Result<IssScript, String> {
    let Some(name) = app_name else {
        return Ok(IssScript {
            app_name: String::new(),
            title: "common_setup.iss".into(),
            text: TEMPLATE.into(),
            path: String::new(),
        });
    };
    let p = cfg
        .products
        .iter()
        .find(|p| p.app_name == name)
        .ok_or_else(|| format!("找不到产品：{name}"))?;
    validate_product(p).map_err(|e| e.to_string())?;
    let info = probe::probe(Path::new(&p.exe_path));
    if !info.ok {
        return Err(info.error);
    }
    let version = version_short(p, &info)?;
    let staging = manifest::staging_dir(&cfg.machine, &p.app_name)?;
    let out = manifest::installer_dir(&cfg.machine)?;
    let path =
        manifest::iss_dir(&cfg.machine, &p.app_name)?.join(format!("{}.iss", p.app_name));
    Ok(IssScript {
        app_name: p.app_name.clone(),
        title: format!("{}.iss", p.app_name),
        text: render(p, &version, &staging, &out),
        path: path.to_string_lossy().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn product() -> ProductConfig {
        ProductConfig {
            app_name: "AlphaTool".into(),
            app_id: "9D5A0C45-3B1E-4B7F-9C2A-8E6D1F0A2B33".into(),
            ..Default::default()
        }
    }

    #[test]
    fn script_supplies_every_define() {
        let script = render(
            &product(),
            "1.2.3",
            Path::new("D:\\QDeploy\\staging\\AlphaTool"),
            Path::new("D:\\QDeploy\\installers"),
        );
        assert_eq!(
            script,
            "#define AppName \"AlphaTool\"\n\
             #define AppGuid \"9D5A0C45-3B1E-4B7F-9C2A-8E6D1F0A2B33\"\n\
             #define AppVersion \"1.2.3\"\n\
             #define SourceDir \"D:\\QDeploy\\staging\\AlphaTool\"\n\
             #define OutputDir \"D:\\QDeploy\\installers\"\n\
             #include \"common_setup.iss\"\n"
        );
    }

    /// The template must not go back to guessing values or including generated neighbours.
    #[test]
    fn template_uses_only_generated_defines() {
        for needle in ["{#AppName}", "{#AppGuid}", "{#AppVersion}", "{#SourceDir}", "{#OutputDir}"] {
            assert!(TEMPLATE.contains(needle), "模板缺 {needle}");
        }
        for gone in ["local.iss", ".deploy.iss", "GetVersionNumbersString"] {
            assert!(!TEMPLATE.contains(gone), "模板还在用 {gone}");
        }
    }

    /// ISPP emits the GUID bare and swallows one closing brace: three opens + two closes is the
    /// only spelling that reaches Inno as AppId={{GUID}. Two opens makes ISCC say the constant
    /// is unterminated, so this line must not be "simplified".
    #[test]
    fn appid_keeps_the_escaped_braces() {
        assert!(
            TEMPLATE.lines().any(|l| l == "AppId={{{#AppGuid}}"),
            "AppId 的花括号转义被改坏了：三个 {{ 两个 }} 才是对的"
        );
    }

    /// The preview and the run must agree on which version name the installer gets.
    #[test]
    fn version_short_prefers_the_override_and_rejects_an_empty_one() {
        let mut p = product();
        let mut info = ProbeInfo { ok: true, version_full: "1.0.0.0".into(), ..Default::default() };
        assert_eq!(version_short(&p, &info).unwrap(), "1.0.0");

        p.version_override = " 2.3.4.5 ".into();
        assert_eq!(version_short(&p, &info).unwrap(), "2.3.4");

        p.version_override = "  ".into();
        info.version_full.clear();
        assert_eq!(
            version_short(&p, &info).unwrap_err(),
            "exe 没有版本资源，且没设版本覆盖"
        );
    }

    #[test]
    fn template_script_needs_no_config() {
        let script = script_for(&Config::default(), None).unwrap();
        assert_eq!(script.title, "common_setup.iss");
        assert_eq!(script.text, TEMPLATE);
        assert!(script.path.is_empty());
        assert!(script.app_name.is_empty());
    }

    #[test]
    fn unknown_products_are_named() {
        let err = script_for(&Config::default(), Some("Nope")).unwrap_err();
        assert!(err.contains("找不到产品：Nope"), "unexpected: {err}");
    }

    #[test]
    fn a_missing_exe_keeps_the_preview_error_readable() {
        let tmp = tempfile::tempdir().unwrap();
        let mut cfg = Config::default();
        cfg.machine.work_root = tmp.path().join("work").to_string_lossy().to_string();
        let mut p = product();
        p.exe_path = tmp.path().join("AlphaTool.exe").to_string_lossy().to_string();
        cfg.products.push(p);

        let err = script_for(&cfg, Some("AlphaTool")).unwrap_err();
        assert!(err.contains("读不到 exe"), "unexpected: {err}");
    }
}
