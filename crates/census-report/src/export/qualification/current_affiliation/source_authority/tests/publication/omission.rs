use super::*;
use census_store::{
    replay_frozen_roster_reviewed_claims_v1, RosterFrozenReviewedClaimsReferenceV1,
    RosterSourceIssuanceStoreError, StoreError,
};
use sha2::{Digest, Sha256};

mod wire;

#[test]
fn controlled_rehashed_review_omission_refuses_against_original_authority_head() -> TestResult {
    let (fixture, expected) = setup::prepare(Case::Conflicted)?;
    let objects = tempfile::tempdir()?;
    let original = tempfile::tempdir()?;
    let candidate = tempfile::tempdir()?;
    let output = tempfile::tempdir()?;
    let dataset = capture(&fixture, objects.path())?;
    witness(&dataset, &expected)?;
    let frozen = original.path().join("authentic.json");
    dataset.save_frozen(&frozen)?;
    let original_bytes = std::fs::read(&frozen)?;
    let original_digest = dataset.archive_digest()?;
    let root = output.path().join("publication");
    let workbook = publish(&fixture, &dataset, &root)?;
    let pointer = std::fs::read_link(root.join("current"))?;
    let verified = published::verify_published(&workbook)?.generation_digest;
    let generation = workbook.parent().ok_or("controlled valid generation")?;
    let manifest = std::fs::read(generation.join("manifest.json"))?;
    let workbook_bytes = std::fs::read(&workbook)?;
    let published_frozen = std::fs::read(generation.join("frozen-input.json"))?;
    let hostile = candidate.path().join("attacker-transport.json");
    dataset.save_frozen(&hostile)?;
    let transport_digest = omit_review(&hostile)?;
    check!(transport_digest != original_digest);
    let rejected = ExportDataset::reopen_frozen_with_digest(&hostile, &transport_digest);
    check!(matches!(
        rejected,
        Err(crate::report::ReportError::Store(
            StoreError::RosterSourceIssuance(RosterSourceIssuanceStoreError::Corrupt(
                "review complete inventory mismatch"
            ))
        ))
    ));
    let refused = admit_save_publish(
        &fixture,
        &hostile,
        &transport_digest,
        &candidate.path().join("admitted.json"),
        &root,
    );
    check!(matches!(
        refused,
        Err(crate::report::ReportError::Store(
            StoreError::RosterSourceIssuance(RosterSourceIssuanceStoreError::Corrupt(
                "review complete inventory mismatch"
            ))
        ))
    ));
    check!(!candidate.path().join("admitted.json").exists());
    check!(eq; std::fs::read_link(root.join("current"))?, pointer);
    check!(eq; published::verify_published(&workbook)?.generation_digest, verified);
    check!(eq; std::fs::read(&frozen)?, original_bytes);
    check!(eq; std::fs::read(generation.join("manifest.json"))?, manifest);
    check!(eq; std::fs::read(&workbook)?, workbook_bytes);
    check!(eq; std::fs::read(generation.join("frozen-input.json"))?, published_frozen);
    check!(eq; admit_save_publish(&fixture, &frozen, &original_digest, &original.path().join("restored.json"), &root)?, workbook);
    check!(eq; published::verify_published(&workbook)?.generation_digest, verified);
    Ok(())
}

fn admit_save_publish(
    fixture: &fixture::Fixture,
    frozen: &Path,
    digest: &str,
    save: &Path,
    root: &Path,
) -> crate::report::ReportResult<PathBuf> {
    let dataset = ExportDataset::reopen_frozen_with_digest(frozen, digest)?;
    dataset.save_frozen(save)?;
    let options = Options {
        grad_year: Some(2027),
        out: Some(root.to_path_buf()),
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: fixture.profile.input().jurisdictions.clone(),
        school_year: fixture.profile.input().contact_school_year,
    };
    let censuses = Censuses::of(&dataset, &fixture.store.out_dir(), &options.jurisdictions)?;
    workbook::build_from(&dataset, &fixture.store, &options, &censuses)
}

fn omit_review(path: &Path) -> TestResult<String> {
    let mut envelope = inspect_wire(path)?;
    let anchors = json!({"data":envelope.get("data"),"lineage":envelope.get("lineage"),"physical":envelope.pointer("/selected_proofs/affiliation/observations"),"family":envelope.pointer("/selected_proofs/affiliation/roster_family")});
    let directory = path.with_file_name(format!(
        "{}.proofs",
        path.file_name()
            .and_then(|name| name.to_str())
            .ok_or("controlled archive filename")?
    ));
    let old_closure = envelope
        .pointer("/selected_proofs/affiliation/archives/digest")
        .and_then(Value::as_str)
        .ok_or("controlled closure")?
        .to_owned();
    let mut closure: Value = serde_json::from_slice(&std::fs::read(directory.join(&old_closure))?)?;
    let old_bundle = closure
        .pointer("/reviewed/digest")
        .and_then(Value::as_str)
        .ok_or("controlled reviewed bundle")?
        .to_owned();
    let original_ref = closure
        .get("reviewed")
        .ok_or("controlled reviewed reference")?
        .clone();
    let omitted = wire::omit_latest(&std::fs::read(directory.join(&old_bundle))?)?;
    let bundle_digest = hash(&omitted);
    let reviewed = closure
        .get_mut("reviewed")
        .and_then(Value::as_object_mut)
        .ok_or("controlled reviewed reference object")?;
    reviewed.insert("digest".into(), json!(bundle_digest));
    reviewed.insert("bytes".into(), json!(omitted.len()));
    for key in [
        "authority_head",
        "current_input_token",
        "store",
        "selected_snapshot_sequence",
        "version",
    ] {
        check!(eq; reviewed.get(key), original_ref.get(key));
    }
    let reference: RosterFrozenReviewedClaimsReferenceV1 =
        serde_json::from_value(Value::Object(reviewed.clone()))?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        replay_frozen_roster_reviewed_claims_v1(&omitted, &reference, &mut grant),
        Err(StoreError::RosterSourceIssuance(
            RosterSourceIssuanceStoreError::Corrupt("review complete inventory mismatch")
        ))
    ));
    let typed: crate::export::qualification::current_affiliation::source_binding::RosterSourceClosureV1 = serde_json::from_value(closure)?;
    let closure_bytes = serde_json::to_vec(&typed)?;
    let closure_digest = hash(&closure_bytes);
    replace_object(&mut envelope, &directory, &old_bundle, &omitted)?;
    replace_object(&mut envelope, &directory, &old_closure, &closure_bytes)?;
    *envelope
        .pointer_mut("/selected_proofs/affiliation/archives")
        .ok_or("controlled affiliation archive")? =
        json!({"digest":closure_digest,"bytes":closure_bytes.len()});
    let descriptor: crate::export::qualification::current_affiliation::FrozenAffiliationDescriptor =
        serde_json::from_value(
            envelope
                .pointer("/selected_proofs/affiliation")
                .ok_or("controlled descriptor")?
                .clone(),
        )?;
    *envelope
        .pointer_mut("/selected_proofs/affiliation_digest")
        .ok_or("controlled descriptor digest")? = json!(descriptor.digest()?);
    check!(eq; json!({"data":envelope.get("data"),"lineage":envelope.get("lineage"),"physical":envelope.pointer("/selected_proofs/affiliation/observations"),"family":envelope.pointer("/selected_proofs/affiliation/roster_family")}), anchors);
    let bytes = serde_json::to_vec(&envelope)?;
    std::fs::write(path, &bytes)?;
    Ok(hash(&bytes))
}

fn replace_object(envelope: &mut Value, directory: &Path, old: &str, bytes: &[u8]) -> TestResult {
    let digest = crate::export::proof_objects::install(directory, bytes)?;
    check!(digest != old);
    let inventory = envelope
        .pointer_mut("/selected_proofs/objects")
        .and_then(Value::as_array_mut)
        .ok_or("controlled object inventory")?;
    check!(inventory.len() <= 16384);
    let entry = inventory
        .iter_mut()
        .find(|entry| entry.get("digest").and_then(Value::as_str) == Some(old))
        .ok_or("controlled retained object")?;
    *entry = json!({"digest":digest,"bytes":bytes.len()});
    inventory.sort_by(|left, right| {
        left.get("digest")
            .and_then(Value::as_str)
            .cmp(&right.get("digest").and_then(Value::as_str))
    });
    std::fs::remove_file(directory.join(old))?;
    Ok(())
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
