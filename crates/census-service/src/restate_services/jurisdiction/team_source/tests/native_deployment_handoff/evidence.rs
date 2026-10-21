use super::TestResult;
use serde::Serialize;
use std::path::PathBuf;

pub(super) struct Evidence(PathBuf);

impl Evidence {
    pub(super) fn new() -> TestResult<Self> {
        let root = tempfile::Builder::new()
            .prefix("native-deployment-handoff-")
            .tempdir()?;
        let path = root.keep();
        println!("native deployment handoff evidence: {}", path.display());
        Ok(Self(path))
    }

    pub(super) fn write(&self, name: &str, value: &impl Serialize) -> TestResult {
        check!(name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-._".contains(&byte)));
        let bytes = serde_json::to_vec_pretty(value)?;
        check!(bytes.len() <= 1_048_576, "native evidence exceeds 1 MiB");
        std::fs::write(self.0.join(name), bytes)?;
        Ok(())
    }
}
