use crate::config::{validate_app_name, MachineConfig, ProductConfig};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use std::path::{Component, Path, PathBuf};

pub fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn too_shallow(root: &Path) -> bool {
    root.components().filter(|c| matches!(c, Component::Normal(_))).count() < 2
}

/// staging dir == <stagingRoot>\<AppName>; the generated iss hands it to the template as SourceDir.
pub fn staging_dir(machine: &MachineConfig, app_name: &str) -> Result<PathBuf, String> {
    validate_app_name(app_name).map_err(|e| e.to_string())?;
    let root = normalize(Path::new(&machine.staging_root));
    if machine.staging_root.trim().is_empty() || too_shallow(&root) {
        return Err(format!("暂存根路径太浅，拒绝操作：{}", root.display()));
    }
    Ok(root.join(app_name))
}

pub fn iss_dir(machine: &MachineConfig, app_name: &str) -> Result<PathBuf, String> {
    Ok(staging_dir(machine, app_name)?.parent().unwrap().join(".iss").join(app_name))
}

pub fn remove_dir(dir: &Path) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    match std::fs::remove_dir_all(dir) {
        Ok(()) => Ok(()),
        Err(_) => {
            std::thread::sleep(std::time::Duration::from_millis(300));
            std::fs::remove_dir_all(dir).map_err(|e| format!("删除 {} 失败：{e}", dir.display()))
        }
    }
}

/// Removes both folders a run owns: <root>\<AppName> and <root>\.iss\<AppName>.
pub fn clear_app_dirs(machine: &MachineConfig, app_name: &str) -> Result<(), String> {
    let staging = staging_dir(machine, app_name)?;
    remove_dir(&staging)?;
    remove_dir(&iss_dir(machine, app_name)?)
}

pub fn apply_includes(p: &ProductConfig, exe_dir: &Path, staging: &Path) -> Result<(), String> {
    for entry in &p.include {
        let src = if Path::new(&entry.source).is_absolute() {
            normalize(Path::new(&entry.source))
        } else {
            normalize(&exe_dir.join(&entry.source))
        };
        if !src.exists() {
            return Err(format!("清单里的源不存在：{}", src.display()));
        }
        let dst = normalize(&staging.join(&entry.target));
        if !dst.starts_with(staging) {
            return Err(format!("清单目标跑出暂存目录了：{}", entry.target));
        }
        if src.is_dir() {
            copy_tree(&src, &dst).map_err(|e| format!("复制 {} 失败：{e}", src.display()))?;
        } else {
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::copy(&src, &dst).map_err(|e| format!("复制 {} 失败：{e}", src.display()))?;
        }
    }
    Ok(())
}

fn copy_tree(src: &Path, dst: &Path) -> std::io::Result<()> {
    for entry in walkdir::WalkDir::new(src) {
        let entry = entry?;
        let rel = entry.path().strip_prefix(src).unwrap();
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

fn build_set(patterns: &[String]) -> Result<GlobSet, String> {
    let mut builder = GlobSetBuilder::new();
    for pat in patterns {
        let normalized = pat.replace('\\', "/");
        let glob = GlobBuilder::new(&normalized)
            .literal_separator(false)
            .backslash_escape(false)
            .build()
            .map_err(|e| format!("排除规则 {pat} 不合法：{e}"))?;
        builder.add(glob);
    }
    builder.build().map_err(|e| e.to_string())
}

/// Deletes whatever the exclude patterns match inside staging. Empty matches are only warnings.
pub fn apply_excludes(p: &ProductConfig, staging: &Path) -> Result<Vec<String>, String> {
    if p.exclude.is_empty() {
        return Ok(Vec::new());
    }
    let set = build_set(&p.exclude)?;
    let mut warnings = Vec::new();
    let mut hits: Vec<PathBuf> = Vec::new();
    for entry in walkdir::WalkDir::new(staging).contents_first(true) {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path == staging {
            continue;
        }
        let rel = path.strip_prefix(staging).unwrap().to_string_lossy().replace('\\', "/");
        if set.is_match(&rel) {
            hits.push(path.to_path_buf());
        }
    }
    let mut matched = vec![false; p.exclude.len()];
    for (i, pat) in p.exclude.iter().enumerate() {
        let normalized = pat.replace('\\', "/");
        if hits.iter().any(|h| {
            let rel = h.strip_prefix(staging).unwrap().to_string_lossy().replace('\\', "/");
            let single = GlobBuilder::new(&normalized).literal_separator(false).backslash_escape(false).build().map(|g| g.compile_matcher().is_match(&rel)).unwrap_or(false);
            single
        }) {
            matched[i] = true;
        }
    }
    for path in &hits {
        if path.is_dir() {
            std::fs::remove_dir_all(path).map_err(|e| format!("删除 {} 失败：{e}", path.display()))?;
        } else if path.exists() {
            std::fs::remove_file(path).map_err(|e| format!("删除 {} 失败：{e}", path.display()))?;
        }
    }
    for (i, ok) in matched.iter().enumerate() {
        if !ok {
            warnings.push(format!("排除规则没匹配到任何文件：{}", p.exclude[i]));
        }
    }
    Ok(warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ManifestEntry;

    fn machine(root: &Path) -> MachineConfig {
        MachineConfig {
            staging_root: root.join("staging").to_string_lossy().to_string(),
            ..Default::default()
        }
    }

    fn product(app: &str) -> ProductConfig {
        ProductConfig { app_name: app.into(), ..Default::default() }
    }

    #[test]
    fn rejects_shallow_root() {
        let m = MachineConfig { staging_root: "C:\\".into(), ..Default::default() };
        assert!(staging_dir(&m, "App").is_err());
    }

    #[test]
    fn rejects_bad_app_name() {
        let tmp = tempfile::tempdir().unwrap();
        let m = machine(tmp.path());
        assert!(staging_dir(&m, "../x").is_err());
        assert!(staging_dir(&m, "").is_err());
    }

    #[test]
    fn copies_files_and_trees_and_blocks_escape() {
        let tmp = tempfile::tempdir().unwrap();
        let m = machine(tmp.path());
        let staging = staging_dir(&m, "App").unwrap();
        std::fs::create_dir_all(&staging).unwrap();
        let exe_dir = tmp.path().join("build");
        std::fs::create_dir_all(exe_dir.join("assets")).unwrap();
        std::fs::write(exe_dir.join("app.ini"), b"x").unwrap();
        std::fs::write(exe_dir.join("assets/logo.txt"), b"y").unwrap();

        let mut p = product("App");
        p.include = vec![
            ManifestEntry { source: "app.ini".into(), target: "config/app.ini".into() },
            ManifestEntry { source: "assets".into(), target: "assets".into() },
        ];
        apply_includes(&p, &exe_dir, &staging).unwrap();
        assert!(staging.join("config/app.ini").exists());
        assert!(staging.join("assets/logo.txt").exists());

        p.include = vec![ManifestEntry { source: "app.ini".into(), target: "../evil".into() }];
        assert!(apply_includes(&p, &exe_dir, &staging).is_err());
    }

    #[test]
    fn excludes_match_and_warn() {
        let tmp = tempfile::tempdir().unwrap();
        let m = machine(tmp.path());
        let staging = staging_dir(&m, "App").unwrap();
        std::fs::create_dir_all(staging.join("plugins/sqldrivers")).unwrap();
        std::fs::write(staging.join("plugins/sqldrivers/qsqlite.dll"), b"x").unwrap();
        std::fs::write(staging.join("app.exe"), b"x").unwrap();

        let mut p = product("App");
        p.exclude = vec!["plugins\\sqldrivers\\*".into(), "nothing\\*".into()];
        let warnings = apply_excludes(&p, &staging).unwrap();
        assert!(!staging.join("plugins/sqldrivers/qsqlite.dll").exists());
        assert!(staging.join("app.exe").exists());
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn clears_staging_and_iss_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let m = machine(tmp.path());
        let staging = staging_dir(&m, "App").unwrap();
        let iss = iss_dir(&m, "App").unwrap();
        std::fs::create_dir_all(staging.join("sub")).unwrap();
        std::fs::write(staging.join("sub/x"), b"x").unwrap();
        std::fs::create_dir_all(&iss).unwrap();
        std::fs::write(iss.join("local.iss"), b"x").unwrap();

        clear_app_dirs(&m, "App").unwrap();
        assert!(!staging.exists());
        assert!(!iss.exists());
    }
}
