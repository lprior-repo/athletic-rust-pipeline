use super::*;
use serde_json::{json, Value};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn encoded(value: &Value) -> TestResult<Vec<u8>> {
    let manifest: format::Manifest = serde_json::from_value(value.clone())?;
    Ok(serde_json::to_vec(&manifest)?)
}

#[test]
fn compiled_identity_is_deterministic_and_retains_original_bytes() -> TestResult {
    let first = compiled_roster_parser_manifest()?;
    let second = compiled_roster_parser_manifest()?;
    check!(first.canonical_bytes() == second.canonical_bytes());
    check!(first.parser_claim()? == second.parser_claim()?);
    let claim = first.parser_claim()?;
    let mut grant = AttachmentReplayGrantV1::new();
    let checked =
        assess_retained_roster_parser_manifest(first.canonical_bytes(), &claim, &mut grant)?;
    check!(checked.retained().canonical_bytes() == first.canonical_bytes());
    check!(checked.retained().semantic_policy() == CurrentRosterAssertion::INTERPRETATION_REVISION);
    check!(claim.artifact().digest().as_str() == hash(first.canonical_bytes()));
    Ok(())
}

#[test]
fn caller_supplied_digests_cannot_establish_compiled_provenance() -> TestResult {
    let compiled = compiled_roster_parser_manifest()?;
    let claim = compiled.parser_claim()?;
    for field in ["artifact", "schema", "configuration", "dependencies"] {
        let mut value = serde_json::to_value(&claim)?;
        let reference = value.get_mut(field).ok_or("parser claim field absent")?;
        *reference
            .get_mut("digest")
            .ok_or("parser claim digest absent")? = json!("a".repeat(64));
        let changed = serde_json::from_value(value)?;
        check!(compiled.require_claim(&changed).is_err());
    }
    Ok(())
}

#[test]
fn altered_source_configuration_lock_and_policy_refuse_current_adapter() -> TestResult {
    let compiled = compiled_roster_parser_manifest()?;
    for pointer in [
        "/source_trees/0/sha256",
        "/configuration/files/0/sha256",
        "/dependencies/sha256",
    ] {
        let mut value: Value = serde_json::from_slice(compiled.canonical_bytes())?;
        *value.pointer_mut(pointer).ok_or("manifest input absent")? = json!("a".repeat(64));
        if pointer == "/source_trees/0/sha256" {
            *value
                .pointer_mut("/schema/source_trees/0/sha256")
                .ok_or("schema input absent")? = json!("a".repeat(64));
        }
        let bytes = encoded(&value)?;
        let retained = RetainedRosterParserManifestV1::decode(&bytes)?;
        let claim = retained.parser_claim()?;
        check!(assess_retained_roster_parser_manifest(
            &bytes,
            &claim,
            &mut AttachmentReplayGrantV1::new()
        )
        .is_err());
        check!(retained.canonical_bytes() == bytes);
    }
    Ok(())
}

#[test]
fn unknown_version_policy_extra_field_and_noncanonical_bytes_refuse() -> TestResult {
    let compiled = compiled_roster_parser_manifest()?;
    for (key, replacement) in [
        ("version", json!(2)),
        ("semantic_policy", json!("unknown/v1")),
        ("extra", json!(true)),
    ] {
        let mut value: Value = serde_json::from_slice(compiled.canonical_bytes())?;
        value
            .as_object_mut()
            .ok_or("manifest not object")?
            .insert(key.into(), replacement);
        check!(RetainedRosterParserManifestV1::decode(&serde_json::to_vec(&value)?).is_err());
    }
    let mut whitespace = compiled.canonical_bytes().to_vec();
    whitespace.push(b'\n');
    check!(RetainedRosterParserManifestV1::decode(&whitespace).is_err());
    check!(RetainedRosterParserManifestV1::decode(&vec![b' '; format::MAX_MANIFEST + 1]).is_err());
    Ok(())
}

#[test]
fn unknown_vcs_is_not_clean_and_vcs_does_not_relabel_content_identity() -> TestResult {
    let compiled = compiled_roster_parser_manifest()?;
    let digest = hash(compiled.canonical_bytes());
    check!(validate::vcs(
        &format::Vcs {
            manifest_sha256: digest.clone(),
            commit: String::new(),
            state: RosterParserWorkingTreeV1::Unknown
        },
        &digest
    )
    .is_err());
    let mut vcs = format::Vcs {
        manifest_sha256: digest.clone(),
        commit: compiled.git_commit().into(),
        state: RosterParserWorkingTreeV1::GitReportedDirty,
    };
    validate::vcs(&vcs, &digest)?;
    vcs.state = RosterParserWorkingTreeV1::GitReportedClean;
    validate::vcs(&vcs, &digest)?;
    check!(validate::vcs(&vcs, &"a".repeat(64)).is_err());
    check!(!std::str::from_utf8(compiled.canonical_bytes())?.contains(compiled.git_commit()));
    Ok(())
}

#[test]
fn duplicate_or_missing_configuration_inputs_refuse() -> TestResult {
    let compiled = compiled_roster_parser_manifest()?;
    let mut value: Value = serde_json::from_slice(compiled.canonical_bytes())?;
    let entries = value
        .pointer_mut("/configuration/environment")
        .and_then(Value::as_array_mut)
        .ok_or("configuration environment absent")?;
    let duplicate = entries
        .first()
        .ok_or("configuration environment empty")?
        .clone();
    entries.push(duplicate);
    check!(RetainedRosterParserManifestV1::decode(&encoded(&value)?).is_err());
    Ok(())
}

#[test]
fn section_references_hash_exact_retained_section_bytes() -> TestResult {
    let compiled = compiled_roster_parser_manifest()?;
    let claim = compiled.parser_claim()?;
    for (reference, bytes) in [
        (claim.schema(), compiled.retained().schema_bytes()?),
        (
            claim.configuration(),
            compiled.retained().configuration_bytes()?,
        ),
        (
            claim.dependencies(),
            compiled.retained().dependencies_bytes()?,
        ),
    ] {
        check!(reference.digest().as_str() == hash(&bytes));
        check!(reference.bytes().get() == u64::try_from(bytes.len())?);
    }
    Ok(())
}

#[test]
fn exhausted_or_failed_manifest_assessment_poison_replay_grant() -> TestResult {
    let compiled = compiled_roster_parser_manifest()?;
    let claim = compiled.parser_claim()?;
    let mut exhausted = AttachmentReplayGrantV1::new();
    check!(exhausted.charge_bytes(usize::MAX).is_err());
    check!(assess_retained_roster_parser_manifest(
        compiled.canonical_bytes(),
        &claim,
        &mut exhausted
    )
    .is_err());
    let mut failed = AttachmentReplayGrantV1::new();
    check!(assess_retained_roster_parser_manifest(b"{}", &claim, &mut failed).is_err());
    check!(failed.ensure_available().is_err());
    Ok(())
}
