use super::*;
use std::path::Component;

pub(super) struct Retained {
    pub path: PathBuf,
    pub root: PathBuf,
    pub bytes: Vec<u8>,
    pub manifest: Value,
    pub inventory: Value,
    pub parser: Vec<u8>,
}

pub(super) fn read() -> TestResult<Retained> {
    let path = PathBuf::from(std::env::var("CENSUS_REVIEWED_ROSTER_RETENTION_JSON")?);
    check!(path.is_absolute());
    check!(eq; std::fs::canonicalize(&path)?, path);
    check!(std::fs::symlink_metadata(&path)?.len() <= 32 * 1024 * 1024);
    let bytes = files::read(&path)?;
    let expected = digest_env("CENSUS_REVIEWED_ROSTER_RETENTION_SHA256")?;
    check!(eq; files::digest(&bytes), expected);
    let manifest: Value = serde_json::from_slice(&bytes)?;
    check!(eq; string(&manifest, "schema")?, "controlled-reviewed-roster-retention/v1");
    check!(eq; string(&manifest, "report_producer_commit")?, files::revision("CENSUS_REVIEWED_ROSTER_REPORT_PRODUCER_COMMIT")?);
    check!(eq; string(&manifest, "report_producer_tree")?, files::revision("CENSUS_REVIEWED_ROSTER_REPORT_PRODUCER_TREE")?);
    let root = path.parent().ok_or("retention parent")?.to_path_buf();
    check!(eq; path.file_name().and_then(|name| name.to_str()), Some("retention.json"));
    let inventory = serde_json::to_value(files::inventory(&root)?)?;
    let actual = inventory
        .as_array()
        .ok_or("retained file inventory")?
        .iter()
        .filter(|item| item.get("path").and_then(Value::as_str) != Some("retention.json"))
        .cloned()
        .collect::<Vec<_>>();
    check!(eq; serde_json::to_value(actual)?, *manifest.get("files").ok_or("declared full inventory")?);
    let cases = manifest
        .get("cases")
        .and_then(Value::as_array)
        .ok_or("retained cases")?;
    check!(eq; cases.len(), 3);
    let names = cases
        .iter()
        .map(|case| string(case, "case"))
        .collect::<TestResult<std::collections::BTreeSet<_>>>()?;
    check!(eq; names, std::collections::BTreeSet::from(["resolved", "conflicted", "missing-owner"]));
    for case in cases {
        let name = string(case, "case")?;
        let archive = relative(&root, string(case, "archive")?)?;
        check!(eq; archive, root.join(name).join("archive/controlled-reviewed-source.json"));
        check!(eq; serde_json::to_value(files::inventory(archive.parent().ok_or("archive parent")?)?)?, *case.get("archive_files").ok_or("archive inventory")?);
        let workbook = relative(&root, string(case, "workbook")?)?;
        check!(workbook.starts_with(root.join(name).join("publication")));
        check!(eq; serde_json::to_value(files::inventory(workbook.parent().ok_or("generation parent")?)?)?, *case.get("publication_files").ok_or("publication inventory")?);
        check!(eq; case.get("source_store_lifecycle").and_then(Value::as_u64), Some(8));
        check!(eq; case.get("qualification_lifecycle").and_then(Value::as_u64), Some(7));
        check!(eq; case.get("selected_proof_version").and_then(Value::as_u64), Some(3));
        check!(eq; case.get("affiliation_proof_version").and_then(Value::as_u64), Some(2));
    }
    check!(eq; string(&manifest, "parser_manifest")?, "parser-manifest.json");
    check!(eq; string(&manifest, "parser_vcs")?, "parser-vcs.json");
    let parser = files::read(&relative(&root, string(&manifest, "parser_manifest")?)?)?;
    check!(eq; files::digest(&parser), string(&manifest, "parser_manifest_sha256")?);
    let vcs: Value = serde_json::from_slice(&files::read(&root.join("parser-vcs.json"))?)?;
    check!(eq; string(&vcs, "commit")?, string(&manifest, "report_producer_commit")?);
    check!(eq; string(&vcs, "state")?, "git_reported_clean");
    check!(eq; string(&vcs, "manifest_sha256")?, files::digest(&parser));
    Ok(Retained {
        path,
        root,
        bytes,
        manifest,
        inventory,
        parser,
    })
}

pub(super) fn relative(root: &Path, value: &str) -> TestResult<PathBuf> {
    let relative = Path::new(value);
    check!(!value.is_empty() && !relative.is_absolute());
    check!(relative
        .components()
        .all(|part| matches!(part, Component::Normal(_))));
    let path = root.join(relative);
    check!(eq; std::fs::canonicalize(&path)?, path);
    Ok(path)
}

pub(super) fn string<'a>(value: &'a Value, field: &str) -> TestResult<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing retained field {field}").into())
}

fn digest_env(name: &str) -> TestResult<String> {
    let value = std::env::var(name)?;
    check!(
        value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    );
    Ok(value)
}
