use crate::runner::StageRun;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRecord {
    pub app_name: String,
    /// unix seconds
    pub at: u64,
    pub duration_ms: u64,
    pub ok: bool,
    pub message: String,
    pub installer: String,
    pub version: String,
    pub staging_kept: String,
    pub stages: Vec<StageRun>,
}

fn path() -> PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    PathBuf::from(base).join("QDeploy").join("history.json")
}

pub fn load() -> Vec<RunRecord> {
    match std::fs::read_to_string(path()) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub fn append(record: RunRecord) {
    let mut all = load();
    all.push(record);
    let cut = all.len().saturating_sub(300);
    all.drain(..cut);
    let p = path();
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&p, serde_json::to_string(&all).unwrap_or_default());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::StageRun;

    #[test]
    fn stage_states_survive_json() {
        let rec = RunRecord {
            app_name: "App".into(),
            at: 1,
            duration_ms: 2,
            ok: false,
            message: "boom".into(),
            installer: String::new(),
            version: "1.2.3".into(),
            staging_kept: "D:\\\\s\\\\App".into(),
            stages: vec![StageRun { name: "预检".into(), ms: 12, state: "done".into() }],
        };
        let text = serde_json::to_string(&rec).unwrap();
        let back: RunRecord = serde_json::from_str(&text).unwrap();
        assert_eq!(back.stages[0].name, "预检");
        assert_eq!(back.app_name, "App");
    }
}
