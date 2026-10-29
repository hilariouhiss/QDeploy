use crate::config::ProductConfig;
use std::path::Path;

/// The installer template ships inside the exe: no file to point at, nothing to misconfigure.
pub const TEMPLATE: &str = include_str!("../templates/common_setup.iss");

fn q(s: &str) -> String {
    format!("\"{}\"", s)
}

/// The one script QDeploy hands to ISPP: every value the template needs, worked out elsewhere.
pub fn render(p: &ProductConfig, version: &str, source_dir: &Path) -> String {
    let mut script = format!(
        "#define AppName {}\n#define AppGuid {}\n#define AppVersion {}\n#define SourceDir {}\n#define OutputDir {}\n",
        q(&p.app_name),
        q(&p.app_id),
        q(version),
        q(&source_dir.to_string_lossy()),
        q(&p.output_dir)
    );
    script.push_str("#include \"common_setup.iss\"\n");
    script
}

#[cfg(test)]
mod tests {
    use super::*;

    fn product() -> ProductConfig {
        ProductConfig {
            app_name: "AlphaTool".into(),
            app_id: "9D5A0C45-3B1E-4B7F-9C2A-8E6D1F0A2B33".into(),
            output_dir: "D:\\QDeploy\\out".into(),
            ..Default::default()
        }
    }

    #[test]
    fn script_supplies_every_define() {
        let script = render(&product(), "1.2.3", Path::new("D:\\QDeploy\\staging\\AlphaTool"));
        assert_eq!(
            script,
            "#define AppName \"AlphaTool\"\n\
             #define AppGuid \"9D5A0C45-3B1E-4B7F-9C2A-8E6D1F0A2B33\"\n\
             #define AppVersion \"1.2.3\"\n\
             #define SourceDir \"D:\\QDeploy\\staging\\AlphaTool\"\n\
             #define OutputDir \"D:\\QDeploy\\out\"\n\
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
    /// only spelling that reaches Inno as `AppId={{GUID}`. Two opens makes ISCC say the constant
    /// is unterminated, so this line must not be "simplified".
    #[test]
    fn appid_keeps_the_escaped_braces() {
        assert!(
            TEMPLATE.lines().any(|l| l == "AppId={{{#AppGuid}}"),
            "AppId 的花括号转义被改坏了：三个 {{ 两个 }} 才是对的"
        );
    }
}
