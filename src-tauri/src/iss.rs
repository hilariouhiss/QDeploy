use crate::config::{MachineConfig, ProductConfig};

pub struct IssFiles {
    pub local: String,
    pub deploy: String,
    pub product: String,
}

fn q(s: &str) -> String {
    format!("\"{}\"", s)
}

/// The four files your common_setup.iss expects: it is ISPP and owns all the real logic.
pub fn render(machine: &MachineConfig, p: &ProductConfig) -> IssFiles {
    let local = format!(
        "#define SourceDirBase {}\n#define OutputDir {}\n",
        q(&machine.staging_root),
        q(&p.output_dir)
    );
    let deploy = "#define VC 0\n".to_string();
    let mut product = format!(
        "#define AppName {}\n#define AppGuid {}\n",
        q(&p.app_name),
        q(&p.app_id)
    );
    if !p.version_override.trim().is_empty() {
        product.push_str(&format!("#define AppVersionFull {}\n", q(p.version_override.trim())));
    }
    product.push_str("#include \"common_setup.iss\"\n");
    IssFiles { local, deploy, product }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn machine() -> MachineConfig {
        MachineConfig {
            staging_root: "D:\\QDeploy\\staging".into(),
            ..Default::default()
        }
    }

    fn product() -> ProductConfig {
        ProductConfig {
            app_name: "AlphaTool".into(),
            app_id: "9D5A0C45-3B1E-4B7F-9C2A-8E6D1F0A2B33".into(),
            output_dir: "D:\\QDeploy\\out".into(),
            ..Default::default()
        }
    }

    #[test]
    fn local_iss_has_paths() {
        let f = render(&machine(), &product());
        assert_eq!(
            f.local,
            "#define SourceDirBase \"D:\\QDeploy\\staging\"\n#define OutputDir \"D:\\QDeploy\\out\"\n"
        );
        assert_eq!(f.deploy, "#define VC 0\n");
    }

    #[test]
    fn product_iss_is_three_lines_without_override() {
        let f = render(&machine(), &product());
        assert!(f.product.contains("#define AppName \"AlphaTool\""));
        assert!(f.product.contains("#define AppGuid \"9D5A0C45-3B1E-4B7F-9C2A-8E6D1F0A2B33\""));
        assert!(!f.product.contains("AppVersionFull"));
        assert!(f.product.ends_with("#include \"common_setup.iss\"\n"));
    }

    #[test]
    fn override_is_emitted_when_set() {
        let mut p = product();
        p.version_override = " 2.3.4.5 ".into();
        let f = render(&machine(), &p);
        assert!(f.product.contains("#define AppVersionFull \"2.3.4.5\""));
    }
}
