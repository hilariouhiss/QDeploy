use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeInfo {
    pub ok: bool,
    pub error: String,
    pub arch_bits: u32,
    pub qt_major: u32,
    pub compiler: String,
    pub version_full: String,
}

pub fn probe(exe: &Path) -> ProbeInfo {
    let mut info = ProbeInfo::default();
    let data = match std::fs::read(exe) {
        Ok(d) => d,
        Err(e) => {
            info.error = format!("读不到 exe：{e}");
            return info;
        }
    };
    let pe = match goblin::pe::PE::parse(&data) {
        Ok(pe) => pe,
        Err(e) => {
            info.error = format!("不是有效的 PE 文件：{e}");
            return info;
        }
    };
    info.arch_bits = match pe.header.coff_header.machine {
        0x8664 => 64,
        0x14c => 32,
        other => {
            info.error = format!("未知的 PE 架构 0x{other:x}");
            return info;
        }
    };
    let (qt_major, compiler) = classify(&import_dlls(&pe));
    info.qt_major = qt_major;
    info.compiler = compiler;
    info.version_full = file_version(exe).unwrap_or_default();
    if qt_major == 0 {
        info.error = "导入表里没有 Qt5Core/Qt6Core，像是静态链接或不是 Qt 程序".into();
        return info;
    }
    if info.compiler.is_empty() {
        info.error = "认不出编译器运行时（MinGW / llvm-mingw）".into();
        return info;
    }
    info.ok = true;
    info
}

/// The DLL names an exe links against; `Import::name` is the symbol, `Import::dll` the library.
fn import_dlls(pe: &goblin::pe::PE<'_>) -> Vec<String> {
    pe.imports.iter().map(|i| i.dll.to_string()).collect()
}

fn classify(imports: &[String]) -> (u32, String) {
    let lower: Vec<String> = imports.iter().map(|s| s.to_ascii_lowercase()).collect();
    let qt_major = if lower.iter().any(|s| s.starts_with("qt6")) {
        6
    } else if lower.iter().any(|s| s.starts_with("qt5")) {
        5
    } else {
        0
    };
    let compiler = if lower.iter().any(|s| s.contains("libc++") || s.contains("libunwind")) {
        "llvm-mingw"
    } else if lower.iter().any(|s| s.contains("libgcc") || s.contains("libstdc++")) {
        "mingw"
    } else if lower.iter().any(|s| s.contains("vcruntime") || s.contains("msvcp")) {
        "msvc"
    } else {
        ""
    };
    (qt_major, compiler.to_string())
}

/// Inno Setup's GetVersionNumbersString returns a.b.c.d; AppVersion drops the last part.
pub fn short_version(full: &str) -> String {
    match full.rfind('.') {
        Some(i) if i > 0 => full[..i].to_string(),
        _ => full.to_string(),
    }
}

#[link(name = "version")]
extern "system" {
    fn GetFileVersionInfoSizeW(name: *const u16, handle: *mut u32) -> u32;
    fn GetFileVersionInfoW(name: *const u16, handle: u32, len: u32, data: *mut u8) -> i32;
    fn VerQueryValueW(block: *const u8, sub: *const u16, out: *mut *mut u8, len: *mut u32) -> i32;
}

fn file_version(path: &Path) -> Option<String> {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let sub: Vec<u16> = "\\".encode_utf16().chain(Some(0)).collect();
    unsafe {
        let mut handle = 0u32;
        let size = GetFileVersionInfoSizeW(wide.as_ptr(), &mut handle);
        if size == 0 {
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        if GetFileVersionInfoW(wide.as_ptr(), 0, size, buf.as_mut_ptr()) == 0 {
            return None;
        }
        let mut ptr: *mut u8 = std::ptr::null_mut();
        let mut len = 0u32;
        if VerQueryValueW(buf.as_ptr(), sub.as_ptr(), &mut ptr, &mut len) == 0 || ptr.is_null() || len < 16 {
            return None;
        }
        let dwords = std::slice::from_raw_parts(ptr as *const u32, 4);
        let (ms, ls) = (dwords[2], dwords[3]);
        let v = format!("{}.{}.{}.{}", ms >> 16, ms & 0xffff, ls >> 16, ls & 0xffff);
        if v == "0.0.0.0" { None } else { Some(v) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_mingw_qt5() {
        let imps = vec!["Qt5Core.dll".into(), "libgcc_s_seh-1.dll".into(), "KERNEL32.dll".into()];
        assert_eq!(classify(&imps), (5, "mingw".into()));
    }

    #[test]
    fn classifies_llvm_qt6() {
        let imps = vec!["Qt6Core.dll".into(), "libc++.dll".into(), "libunwind.dll".into()];
        assert_eq!(classify(&imps), (6, "llvm-mingw".into()));
    }

    #[test]
    fn classifies_nothing() {
        let imps = vec!["KERNEL32.dll".into()];
        assert_eq!(classify(&imps), (0, "".into()));
    }

    #[test]
    fn shortens_versions() {
        assert_eq!(short_version("1.2.3.4"), "1.2.3");
        assert_eq!(short_version("1.2.3"), "1.2");
        assert_eq!(short_version("7"), "7");
        assert_eq!(short_version(""), "");
    }

    #[test]
    fn reads_pe_header_of_a_real_binary() {
        let p = Path::new("C:\\Windows\\System32\\notepad.exe");
        if !p.exists() {
            return;
        }
        let info = probe(p);
        // notepad is not a Qt app, so ok stays false; the PE header must still parse
        assert_eq!(info.arch_bits, 64);
        assert!(!info.ok);
    }

    /// Regression: classify() must see DLL names, not the symbols imported from them.
    #[test]
    fn import_dlls_are_library_names() {
        let data = std::fs::read(std::env::current_exe().unwrap()).unwrap();
        let pe = goblin::pe::PE::parse(&data).unwrap();
        let dlls = import_dlls(&pe);
        assert!(!dlls.is_empty());
        assert!(
            dlls.iter().all(|d| d.to_ascii_lowercase().ends_with(".dll")),
            "got {dlls:?}"
        );
    }

    #[test]
    fn reads_windows_version_resource() {
        let p = Path::new("C:\\Windows\\System32\\kernel32.dll");
        if !p.exists() {
            return;
        }
        let v = file_version(p).expect("kernel32 has a version resource");
        assert_eq!(v.split('.').count(), 4, "got {v}");
        assert_ne!(v, "0.0.0.0");
    }
}
