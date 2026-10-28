use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Kit {
    pub id: String,
    pub qt_version: String,
    pub arch: String,
    pub compiler: String,
    pub windeployqt: String,
}

/// Scans <root>/<version>/<kit>/bin/windeployqt.exe, the layout Qt's installer produces.
pub fn scan(qt_root: &str) -> Vec<Kit> {
    let mut out = Vec::new();
    let root = Path::new(qt_root);
    let Ok(versions) = std::fs::read_dir(root) else {
        return out;
    };
    for version in versions.flatten() {
        if !version.path().is_dir() {
            continue;
        }
        let qt_version = version.file_name().to_string_lossy().to_string();
        if !qt_version.starts_with(|c: char| c.is_ascii_digit()) {
            continue;
        }
        let Ok(kits) = std::fs::read_dir(version.path()) else {
            continue;
        };
        for kit in kits.flatten() {
            let windeployqt = kit.path().join("bin").join("windeployqt.exe");
            if !windeployqt.is_file() {
                continue;
            }
            let dir_name = kit.file_name().to_string_lossy().to_lowercase();
            let compiler = if dir_name.contains("llvm-mingw") || dir_name.contains("llvm") {
                "llvm-mingw"
            } else if dir_name.contains("mingw") {
                "mingw"
            } else if dir_name.contains("msvc") {
                "msvc"
            } else {
                continue;
            };
            let arch = if dir_name.ends_with("64") { "64" } else { "32" };
            out.push(Kit {
                id: format!("{qt_version}-{compiler}-{arch}"),
                qt_version: qt_version.clone(),
                arch: arch.into(),
                compiler: compiler.into(),
                windeployqt: windeployqt.to_string_lossy().to_string(),
            });
        }
    }
    out.sort_by(|a, b| b.qt_version.cmp(&a.qt_version));
    out
}

pub fn major(version: &str) -> u32 {
    version.split('.').next().and_then(|s| s.parse().ok()).unwrap_or(0)
}

/// Picks the single kit matching the exe, or the forced one.
pub fn resolve(kits: &[Kit], qt_major: u32, arch: &str, compiler: &str, forced: &str) -> Result<Kit, String> {
    if !forced.trim().is_empty() && forced != "auto" {
        return kits
            .iter()
            .find(|k| k.id == forced)
            .cloned()
            .ok_or_else(|| format!("手动指定的套件 {forced} 不在这台机器的 Qt 根下"));
    }
    if kits.is_empty() {
        return Err("Qt 安装根下没有扫描到任何套件，请检查设置页的 Qt 安装根".into());
    }
    let hits: Vec<&Kit> = kits
        .iter()
        .filter(|k| major(&k.qt_version) == qt_major && k.arch == arch && k.compiler == compiler)
        .collect();
    match hits.len() {
        1 => Ok(hits[0].clone()),
        0 => Err(format!(
            "没有匹配的套件（Qt{qt_major} / {arch} 位 / {compiler}）。可用：{}",
            kits.iter().map(|k| k.id.clone()).collect::<Vec<_>>().join(", ")
        )),
        _ => Err(format!(
            "匹配到多个套件（{}），请在产品里手动指定",
            hits.iter().map(|k| k.id.clone()).collect::<Vec<_>>().join(", ")
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(p: &std::path::Path) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b"x").unwrap();
    }

    #[test]
    fn scans_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        touch(&root.join("5.15.2/mingw81_64/bin/windeployqt.exe"));
        touch(&root.join("6.12.0/llvm-mingw_64/bin/windeployqt.exe"));
        touch(&root.join("6.12.0/Src/README.txt"));
        let kits = scan(root.to_str().unwrap());
        assert_eq!(kits.len(), 2);
        assert!(kits.iter().any(|k| k.id == "5.15.2-mingw-64"));
        assert!(kits.iter().any(|k| k.id == "6.12.0-llvm-mingw-64"));
    }

    #[test]
    fn empty_root() {
        assert!(scan("Z:\\no\\such\\dir").is_empty());
    }

    #[test]
    fn resolve_paths() {
        let kits = vec![
            Kit { id: "5.15.2-mingw-64".into(), qt_version: "5.15.2".into(), arch: "64".into(), compiler: "mingw".into(), windeployqt: "a".into() },
            Kit { id: "6.12.0-llvm-mingw-64".into(), qt_version: "6.12.0".into(), arch: "64".into(), compiler: "llvm-mingw".into(), windeployqt: "b".into() },
        ];
        assert_eq!(resolve(&kits, 5, "64", "mingw", "auto").unwrap().id, "5.15.2-mingw-64");
        assert_eq!(resolve(&kits, 6, "64", "llvm-mingw", "").unwrap().id, "6.12.0-llvm-mingw-64");
        assert!(resolve(&kits, 6, "64", "mingw", "auto").is_err());
        assert!(resolve(&kits, 5, "64", "mingw", "9.9.9-mingw-64").is_err());
    }
}
