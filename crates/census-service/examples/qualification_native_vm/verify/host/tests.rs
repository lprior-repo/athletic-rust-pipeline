use super::*;

#[test]
fn repeated_binary_entries_cannot_replace_missing_executables() -> Result<()> {
    let root = tempfile::tempdir()?;
    let payload = root.path();
    let names = ["qualification", "census-serve", "restate-server"];
    let mut entries = Vec::new();
    for name in names {
        std::fs::write(payload.join(name), name.as_bytes())?;
        entries.push(json!({"binary": name, "sha256": artifacts::sha(name.as_bytes())}));
    }
    verify_binaries(&json!({"binaries": entries}), payload)?;
    let repeated = json!({
        "binary": "qualification",
        "sha256": artifacts::sha(b"qualification")
    });
    let duplicated = json!({"binaries": [repeated.clone(), repeated.clone(), repeated]});
    ensure!(
        verify_binaries(&duplicated, payload).is_err(),
        "duplicate roles replaced required executables"
    );
    Ok(())
}

#[test]
fn same_length_executable_changes_invalidate_host_binding() -> Result<()> {
    let root = tempfile::tempdir()?;
    let payload = root.path();
    let mut entries = Vec::new();
    for name in ["qualification", "census-serve", "restate-server"] {
        std::fs::write(payload.join(name), b"original")?;
        entries.push(json!({"binary": name, "sha256": artifacts::sha(b"original")}));
    }
    let manifest = json!({"binaries": entries});
    verify_binaries(&manifest, payload)?;
    std::fs::write(payload.join("census-serve"), b"tampered")?;
    ensure!(
        verify_binaries(&manifest, payload).is_err(),
        "changed executable retained its host binding"
    );
    Ok(())
}
