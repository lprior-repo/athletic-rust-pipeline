use super::*;

#[test]
fn retained_http_output_publication_seal_and_empty_nested_artifacts_refuse_without_mutation(
) -> TestResult {
    for relative in [
        "http/orphan.body",
        "out/workbook.xlsx",
        "out/publication/current",
        "out/seal.json",
        "entities/retained.jsonl",
        "journal/retained.jsonl",
        "capture/retained.body",
        "captures/retained.body",
        "http/archive",
    ] {
        let root = tempfile::tempdir()?;
        let config = prepare(root.path())?;
        let retained = root.path().join(relative);
        if relative == "http/archive" {
            std::fs::create_dir(&retained)?;
        } else {
            std::fs::create_dir_all(retained.parent().ok_or("parent")?)?;
            std::fs::write(&retained, b"retained-artifact")?;
        }
        let metadata = std::fs::symlink_metadata(&retained)?;
        let contents = if metadata.is_file() {
            Some(std::fs::read(&retained)?)
        } else {
            None
        };
        let mut control = Store::installation_control_existing(root.path())?;
        let before = witness(&control.store)?;
        if !matches!(
            control.enroll(&config),
            Err(StoreError::Installation(
                InstallationError::HistoricalStatePresent {
                    kind: HistoricalStateKind::Artifacts
                }
            ))
        ) {
            return Err(format!("artifact {relative} accepted").into());
        }
        equal(witness(&control.store)?, before)?;
        let after = std::fs::symlink_metadata(&retained)?;
        equal(after.file_type(), metadata.file_type())?;
        equal(after.len(), metadata.len())?;
        equal(after.modified()?, metadata.modified()?)?;
        if let Some(contents) = contents {
            equal(std::fs::read(&retained)?, contents)?;
        }
    }
    Ok(())
}

#[test]
fn known_artifact_root_as_regular_file_refuses_without_reading_or_overwriting_it() -> TestResult {
    let root = tempfile::tempdir()?;
    let config = prepare(root.path())?;
    let http = root.path().join("http");
    std::fs::remove_dir(&http)?;
    std::fs::write(&http, b"retained-root-file")?;
    let mut control = Store::installation_control_existing(root.path())?;
    let before = witness(&control.store)?;
    if !matches!(
        control.enroll(&config),
        Err(StoreError::Installation(
            InstallationError::HistoricalStatePresent {
                kind: HistoricalStateKind::Artifacts
            }
        ))
    ) {
        return Err("regular-file artifact root accepted".into());
    }
    equal(std::fs::read(&http)?, b"retained-root-file".to_vec())?;
    equal(witness(&control.store)?, before)
}

#[cfg(unix)]
#[test]
fn known_artifact_root_symlink_and_publication_pointer_refuse_without_following_them() -> TestResult
{
    for link_root in [true, false] {
        let root = tempfile::tempdir()?;
        let config = prepare(root.path())?;
        let external = tempfile::tempdir()?;
        let sentinel = external.path().join("sentinel");
        std::fs::write(&sentinel, b"outside-retained")?;
        let link = if link_root {
            let http = root.path().join("http");
            std::fs::remove_dir(&http)?;
            http
        } else {
            let parent = root.path().join("out/publication");
            std::fs::create_dir(&parent)?;
            parent.join("current")
        };
        std::os::unix::fs::symlink(external.path(), &link)?;
        let target_before = std::fs::read_link(&link)?;
        let mut control = Store::installation_control_existing(root.path())?;
        let before = witness(&control.store)?;
        if !matches!(
            control.enroll(&config),
            Err(StoreError::Installation(
                InstallationError::HistoricalStatePresent {
                    kind: HistoricalStateKind::Artifacts
                }
            ))
        ) {
            return Err("symlink artifact accepted".into());
        }
        equal(std::fs::read_link(&link)?, target_before)?;
        equal(std::fs::read(&sentinel)?, b"outside-retained".to_vec())?;
        equal(witness(&control.store)?, before)?;
    }
    Ok(())
}
