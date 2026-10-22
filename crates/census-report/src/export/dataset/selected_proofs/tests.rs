use super::{identity, objects};
use crate::export::proof_objects;
use serde_json::json;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn copied_objects_survive_original_directory_removal() -> TestResult {
    let source = tempfile::tempdir()?;
    let target = tempfile::tempdir()?;
    let first = proof_objects::install(source.path(), b"first selected proof")?;
    let second = proof_objects::install(source.path(), b"second selected proof")?;
    let inventory = objects::inventory(source.path())?;
    let copied = target.path().join("proofs");
    objects::copy(source.path(), &copied, &inventory)?;
    objects::copy(source.path(), &copied, &inventory)?;
    source.close()?;
    objects::verify(&copied, &inventory)?;
    check!(eq; proof_objects::read(&copied, &first, 100)?, b"first selected proof");
    check!(eq; proof_objects::read(&copied, &second, 100)?, b"second selected proof");
    Ok(())
}

#[test]
fn missing_tampered_and_unlisted_objects_refuse_copy() -> TestResult {
    let source = tempfile::tempdir()?;
    let target = tempfile::tempdir()?;
    let digest = proof_objects::install(source.path(), b"retained bytes")?;
    let inventory = objects::inventory(source.path())?;
    std::fs::write(source.path().join(&digest), b"tampered bytes")?;
    check!(objects::copy(source.path(), target.path(), &inventory).is_err());
    std::fs::remove_file(source.path().join(&digest))?;
    check!(objects::copy(source.path(), target.path(), &inventory).is_err());
    proof_objects::install(source.path(), b"retained bytes")?;
    proof_objects::install(source.path(), b"unlisted source")?;
    check!(objects::copy(source.path(), target.path(), &inventory).is_err());
    Ok(())
}

#[test]
fn unlisted_destination_is_preserved_and_refused() -> TestResult {
    let source = tempfile::tempdir()?;
    let target = tempfile::tempdir()?;
    proof_objects::install(source.path(), b"retained bytes")?;
    let inventory = objects::inventory(source.path())?;
    let foreign = proof_objects::install(target.path(), b"foreign bytes")?;
    check!(objects::copy(source.path(), target.path(), &inventory).is_err());
    check!(eq; proof_objects::read(target.path(), &foreign, 100)?, b"foreign bytes");
    Ok(())
}

#[test]
fn inventory_bounds_are_exact_and_canonical() -> TestResult {
    let mut references = vec![objects::Reference {
        digest: format!("{:064x}", 0),
        bytes: objects::MAX_BYTES,
    }];
    objects::validate(&references)?;
    references.push(objects::Reference {
        digest: format!("{:064x}", 1),
        bytes: 1,
    });
    check!(objects::validate(&references).is_err());
    let duplicate = objects::Reference {
        digest: "a".repeat(64),
        bytes: 1,
    };
    check!(objects::validate(&[duplicate.clone(), duplicate]).is_err());
    check!(objects::validate(&[]).is_err());
    for reference in [
        objects::Reference {
            digest: "A".repeat(64),
            bytes: 1,
        },
        objects::Reference {
            digest: "a".repeat(64),
            bytes: 0,
        },
        objects::Reference {
            digest: "a".repeat(64),
            bytes: proof_objects::MAX_OBJECT_BYTES
                .checked_add(1)
                .ok_or("object ceiling overflow")?,
        },
    ] {
        check!(objects::validate(&[reference]).is_err());
    }
    Ok(())
}

#[test]
fn object_metadata_limits_are_admitted_before_read() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("a".repeat(64));
    let file = std::fs::File::create(&path)?;
    file.set_len(
        proof_objects::MAX_OBJECT_BYTES
            .checked_add(1)
            .ok_or("object ceiling overflow")?,
    )?;
    check!(objects::inventory(directory.path()).is_err());
    Ok(())
}

#[test]
fn identity_bundle_chunk_count_and_boundaries_are_exact() -> TestResult {
    let chunk_bytes = 4 * 1024 * 1024;
    let max_bytes = census_store::MAX_FROZEN_PUBLIC_IDENTITY_BYTES;
    let max_chunks = max_bytes.checked_div(chunk_bytes).ok_or("chunk division")?;
    let chunk = json!({ "digest": "a".repeat(64), "bytes": chunk_bytes });
    let boundary = json!({ "digest": "b".repeat(64), "bytes": max_bytes, "chunks": vec![chunk.clone(); max_chunks] });
    let valid: identity::Bundle = serde_json::from_value(boundary.clone())?;
    valid.validate()?;
    let mut changed = boundary.clone();
    changed["bytes"] = json!(max_bytes
        .checked_add(1)
        .ok_or("bundle byte ceiling overflow")?);
    check!(serde_json::from_value::<identity::Bundle>(changed)?
        .validate()
        .is_err());
    let mut changed = boundary.clone();
    changed["chunks"] = json!(vec![
        chunk;
        max_chunks
            .checked_add(1)
            .ok_or("chunk ceiling overflow")?
    ]);
    check!(serde_json::from_value::<identity::Bundle>(changed).is_err());
    let mut changed = boundary;
    changed["chunks"][0]["bytes"] = json!(chunk_bytes
        .checked_sub(1)
        .ok_or("chunk boundary underflow")?);
    check!(serde_json::from_value::<identity::Bundle>(changed)?
        .validate()
        .is_err());
    Ok(())
}

#[test]
fn identity_bundle_replays_without_live_store() -> TestResult {
    let fixture = crate::export::public_identity::tests::fixture::source_births()?;
    let snapshot = fixture.store.snapshot();
    let frozen = snapshot.freeze_public_identity_v2()?;
    let expected = frozen.replay()?;
    let directory = tempfile::tempdir()?;
    let retained = identity::Bundle::capture(directory.path(), &frozen)?;
    let wire = serde_json::to_vec(&retained)?;
    let retained: identity::Bundle = serde_json::from_slice(&wire)?;
    drop(snapshot);
    drop(fixture);
    let replayed = retained.replay(directory.path())?;
    check!(eq; replayed.head(), expected.head());
    check!(eq; replayed.store_identity(), expected.store_identity());
    check!(eq; replayed.snapshot_sequence(), expected.snapshot_sequence());
    check!(eq; replayed.qualification_version(), expected.qualification_version());
    check!(eq; replayed.qualification_version(), census_domain::public_identity_qualification::PublicIdentityQualificationVersion::RetainedAuthorityGraphV3);
    check!(matches!(
        retained.replay_historical(directory.path()),
        Err(crate::report::ReportError::Store(_))
    ));
    Ok(())
}

fn historical_identity_bundle(directory: &std::path::Path) -> TestResult<identity::Bundle> {
    let identity = census_domain::model::StoreIdentity::parse(&"a".repeat(64))?;
    let declaration =
        census_domain::public_identity_qualification::PublicIdentityQualificationV2::new(
            identity.clone(),
        );
    let head = census_domain::public_identity_history::genesis(&identity)?;
    let bytes = serde_json::to_vec(&json!({
        "version": 1,
        "store_identity": identity,
        "revision": {"evidence_generation": 0, "derived_generation": 0, "snapshot_sequence": 1},
        "declaration": serde_json::to_string(&declaration)?,
        "head": serde_json::to_string(&head)?,
        "history": "[]", "operations": [], "contexts": [],
    }))?;
    let digest = proof_objects::install(directory, &bytes)?;
    Ok(serde_json::from_value(json!({
        "digest": digest, "bytes": bytes.len(),
        "chunks": [{"digest": digest, "bytes": bytes.len()}],
    }))?)
}

#[test]
fn frozen_v1_is_explicitly_read_only_and_never_falls_back_from_graph_replay() -> TestResult {
    let directory = tempfile::tempdir()?;
    let bundle = historical_identity_bundle(directory.path())?;
    let replayed = bundle.replay_historical(directory.path())?;
    check!(eq; replayed.qualification_version(), census_domain::public_identity_qualification::PublicIdentityQualificationVersion::RetainedAuthorityContextV2);
    check!(eq; replayed.head().sequence, 0);
    check!(eq; replayed.public_ids().count(), 0);
    check!(matches!(
        bundle.replay(directory.path()),
        Err(crate::report::ReportError::Store(_))
    ));
    Ok(())
}

#[test]
fn selected_proof_versions_preserve_exact_numeric_wire_and_refuse_unknowns() -> TestResult {
    for (wire, expected) in [
        (1, super::Version::HistoricalContextV1),
        (2, super::Version::RetainedGraphV2),
        (3, super::Version::ReviewedRosterGraphV3),
    ] {
        let decoded: super::Version = serde_json::from_value(json!(wire))?;
        check!(eq; decoded, expected);
        check!(eq; serde_json::to_value(decoded)?, json!(wire));
    }
    for value in [json!(0), json!(4), json!(u32::MAX), json!(-1), json!("2")] {
        check!(serde_json::from_value::<super::Version>(value).is_err());
    }
    Ok(())
}

#[test]
fn identity_chunk_digest_and_bundle_digest_are_independently_enforced() -> TestResult {
    let fixture = crate::export::public_identity::tests::fixture::source_births()?;
    let frozen = fixture.store.snapshot().freeze_public_identity_v2()?;
    let directory = tempfile::tempdir()?;
    let retained = identity::Bundle::capture(directory.path(), &frozen)?;
    let mut wire = serde_json::to_value(&retained)?;
    wire["digest"] = json!("f".repeat(64));
    let changed: identity::Bundle = serde_json::from_value(wire)?;
    check!(changed.replay(directory.path()).is_err());
    let first = retained.chunks().first().ok_or("identity chunk")?;
    std::fs::write(directory.path().join(&first.digest), b"changed")?;
    check!(retained.replay(directory.path()).is_err());
    Ok(())
}

#[test]
fn object_and_bundle_wires_refuse_unknown_fields() -> TestResult {
    let reference = json!({ "digest": "a".repeat(64), "bytes": 1, "untrusted": true });
    check!(serde_json::from_value::<objects::Reference>(reference).is_err());
    let bundle = json!({ "digest": "a".repeat(64), "bytes": 1, "chunks": [], "untrusted": true });
    check!(serde_json::from_value::<identity::Bundle>(bundle).is_err());
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_objects_directories_and_ancestors_are_refused() -> TestResult {
    let source = tempfile::tempdir()?;
    let target = tempfile::tempdir()?;
    let digest = proof_objects::install(source.path(), b"retained bytes")?;
    let inventory = objects::inventory(source.path())?;
    std::os::unix::fs::symlink(source.path().join(&digest), target.path().join(&digest))?;
    check!(objects::verify(target.path(), &inventory).is_err());
    let links = tempfile::tempdir()?;
    let alias = links.path().join("alias");
    std::os::unix::fs::symlink(source.path(), &alias)?;
    check!(objects::verify(&alias, &inventory).is_err());
    let nested = source.path().join("nested");
    std::fs::create_dir(&nested)?;
    check!(objects::require_empty(&alias.join("nested")).is_err());
    Ok(())
}

#[test]
fn bare_relative_destination_is_valid_without_changing_working_directory() -> TestResult {
    let source = tempfile::tempdir()?;
    let target = tempfile::Builder::new()
        .prefix("selected-proof-relative-")
        .tempdir_in(".")?;
    let destination = std::path::Path::new(target.path().file_name().ok_or("relative target")?);
    std::fs::remove_dir(target.path())?;
    proof_objects::install(source.path(), b"relative retained proof")?;
    let inventory = objects::inventory(source.path())?;
    objects::copy(source.path(), destination, &inventory)?;
    objects::verify(destination, &inventory)?;
    target.close()?;
    Ok(())
}

#[test]
fn v1_bundle_reserialization_does_not_acquire_graph_authority() -> TestResult {
    let directory = tempfile::tempdir()?;
    let historical = historical_identity_bundle(directory.path())?;
    let original = serde_json::to_vec(&historical)?;
    let current: identity::Bundle = serde_json::from_slice(&original)?;
    check!(eq; serde_json::to_vec(&current)?, original);
    let old = current.replay_historical(directory.path())?;
    check!(eq; old.qualification_version(), census_domain::public_identity_qualification::PublicIdentityQualificationVersion::RetainedAuthorityContextV2);
    check!(matches!(
        current.replay(directory.path()),
        Err(crate::report::ReportError::Store(_))
    ));
    Ok(())
}
mod large;
