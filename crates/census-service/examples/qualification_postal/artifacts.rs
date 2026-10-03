use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::Path;

pub(super) type Result<T, E = anyhow::Error> = std::result::Result<T, E>;

pub(super) fn sha256(body: &[u8]) -> String {
    format!("{:x}", Sha256::digest(body))
}

pub(super) fn write_json(path: &Path, value: &Value) -> Result<()> {
    write_new(path, &serde_json::to_vec_pretty(value)?)
}

pub(super) fn write_new(path: &Path, body: &[u8]) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(body)?;
    Ok(file.sync_all()?)
}

pub(super) fn save_sources(root: &Path) -> Result<Value> {
    let sources: [(&str, &[u8]); 7] = [
        (
            "qualification_postal.rs",
            include_bytes!("../qualification_postal.rs"),
        ),
        ("qualification_postal/mod.rs", include_bytes!("mod.rs")),
        (
            "qualification_postal/acquisition.rs",
            include_bytes!("acquisition.rs"),
        ),
        (
            "qualification_postal/artifacts.rs",
            include_bytes!("artifacts.rs"),
        ),
        ("qualification_postal/cli.rs", include_bytes!("cli.rs")),
        (
            "qualification_postal/postal.rs",
            include_bytes!("postal.rs"),
        ),
        (
            "qualification_postal/publication.rs",
            include_bytes!("publication.rs"),
        ),
    ];
    std::fs::create_dir(root.join("qualification_postal"))?;
    let mut evidence: [Value; 7] = std::array::from_fn(|_| Value::Null);
    evidence
        .iter_mut()
        .zip(sources)
        .try_for_each(|(entry, (name, body))| -> Result<()> {
            let path = root.join(name);
            write_new(&path, body)?;
            *entry = json!({"path": path, "bytes": body.len(), "sha256": sha256(body)});
            Ok(())
        })?;
    Ok(serde_json::to_value(evidence)?)
}
