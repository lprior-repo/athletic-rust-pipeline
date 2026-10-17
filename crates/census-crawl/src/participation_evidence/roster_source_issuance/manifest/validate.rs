use super::{format, invalid};
use census_store::{StoreError, StoreResult};

pub(super) fn manifest(value: &format::Manifest) -> StoreResult<()> {
    if value.version != format::VERSION || value.semantic_policy != format::POLICY
        || value.closure_policy != format::CLOSURE
        || value.provenance_scope != "compiled-source-config-lock-inputs-not-binary-or-resolved-features/v1"
        || value.schema.contract != "compiled-rust-types-with-entire-source-closure/v1"
        || value.schema.source_trees != value.source_trees
        || value.source_trees.len() != format::SOURCE_ROOTS.len()
    {
        return Err(invalid("unsupported roster parser manifest contract"));
    }
    let (bytes, files) = value.source_trees.iter().zip(format::SOURCE_ROOTS)
        .try_fold((0_u64, 0_usize), |(bytes, files), (tree, path)| {
            digest(&tree.sha256)?;
            if tree.path != path || tree.bytes == 0 || tree.files == 0 {
                return Err(invalid("roster parser source closure differs"));
            }
            Ok((bytes.checked_add(tree.bytes).ok_or(StoreError::CounterOverflow)?,
                files.checked_add(usize::try_from(tree.files).map_err(|_| StoreError::CounterOverflow)?)
                    .ok_or(StoreError::CounterOverflow)?))
        })?;
    if bytes > format::MAX_SOURCE_BYTES || files > format::MAX_FILES {
        return Err(invalid("roster parser source closure capacity exceeded"));
    }
    configuration(&value.configuration)?;
    input(&value.dependencies, "Cargo.lock", format::MAX_FILE_BYTES)?;
    Ok(())
}

fn configuration(value: &format::Configuration) -> StoreResult<()> {
    if value.files.len() != format::CONFIG_FILES.len() || value.environment.len() > 128 {
        return Err(invalid("roster parser configuration count differs"));
    }
    value.files.iter().zip(format::CONFIG_FILES)
        .try_for_each(|(entry, path)| input(entry, path, format::MAX_FILE_BYTES))?;
    input(&value.compiler, "rustc-version-verbose", 16 * 1024)?;
    let mut previous: Option<&str> = None;
    for entry in &value.environment {
        digest(&entry.sha256)?;
        if entry.path.len() > 128 || entry.bytes > 16 * 1024
            || !entry.path.bytes().all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
            || !environment_name(&entry.path)
            || previous.is_some_and(|name| name >= entry.path.as_str())
        {
            return Err(invalid("roster parser environment evidence invalid"));
        }
        previous = Some(&entry.path);
    }
    for name in ["TARGET", "HOST", "PROFILE", "OPT_LEVEL", "DEBUG", "CARGO_ENCODED_RUSTFLAGS"] {
        if !value.environment.iter().any(|entry| entry.path == name) {
            return Err(invalid("roster parser required configuration missing"));
        }
    }
    Ok(())
}

fn environment_name(name: &str) -> bool {
    name.starts_with("CARGO_CFG_") || name.starts_with("CARGO_FEATURE_")
        || matches!(name, "TARGET" | "HOST" | "PROFILE" | "OPT_LEVEL" | "DEBUG"
            | "RUSTFLAGS" | "CARGO_ENCODED_RUSTFLAGS" | "RUSTC_WRAPPER" | "RUSTC_WORKSPACE_WRAPPER")
}

fn input(value: &format::Input, path: &str, limit: u64) -> StoreResult<()> {
    digest(&value.sha256)?;
    if value.path != path || value.bytes == 0 || value.bytes > limit {
        return Err(invalid("roster parser input evidence invalid"));
    }
    Ok(())
}

fn digest(value: &str) -> StoreResult<()> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
        return Err(invalid("roster parser source digest invalid"));
    }
    Ok(())
}

pub(super) fn vcs(value: &format::Vcs, manifest_sha256: &str) -> StoreResult<()> {
    if value.manifest_sha256 != manifest_sha256
        || value.state == format::RosterParserWorkingTreeV1::Unknown
        || ![40, 64].contains(&value.commit.len())
        || !value.commit.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid("compiled roster parser Git provenance unavailable"));
    }
    Ok(())
}
