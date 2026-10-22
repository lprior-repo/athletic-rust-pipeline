use super::{options, TestResult};
use crate::export::ExportDataset;
use crate::workbook;
use census_store::{
    publish::{Digest, Root},
    Store,
};
use serde_json::Value;

pub(in crate::workbook::publication) mod reconstructed;

#[test]
fn reconstructed_revision_one_policy_thirteen_inventory_survives_new_atomic_switch() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let store = Store::open(temporary.path().join("store"))?;
    let root = store.out_dir().join("publication");
    let legacy = reconstructed::install(&store, &root)?;
    let historical = reconstructed::bytes(&legacy)?;
    let (selected, manifest) = super::super::manifest::current_generation(&root)?;
    check!(eq; selected, std::fs::canonicalize(&legacy)?);
    check!(eq; manifest.lineage.schema_revision, 2);
    check!(eq; manifest.lineage.policy_revision, 13);
    check!(eq; reconstructed::bytes(&legacy)?, historical);
    let error = super::super::manifest::verify_published(&legacy.join("workbook.xlsx"))
        .err()
        .ok_or("legacy input must not be reinterpreted")?;
    check!(error
        .to_string()
        .contains("unsupported or malformed frozen export lineage"));
    check!(eq; reconstructed::bytes(&legacy)?, historical);
    let dataset = ExportDataset::load(&store)?;
    super::super::fence(&root, &dataset)?;
    let previous = super::super::pointer::read(&root)?;
    check!(eq; previous, Root::Bundle(Digest::parse(&manifest.generation_digest)?));
    let published = workbook::build(&store, &options()?)?;
    check!(ne; published.parent(), Some(legacy.as_path()));
    let generation = published.parent().ok_or("new generation")?;
    let current = super::super::pointer::read(&root)?;
    let current_manifest: Value =
        serde_json::from_slice(&std::fs::read(generation.join("manifest.json"))?)?;
    let digest = current_manifest
        .get("generation_digest")
        .and_then(Value::as_str)
        .ok_or("new digest")?;
    check!(eq; current, Root::Bundle(Digest::parse(digest)?));
    check!(eq; reconstructed::bytes(&legacy)?, historical);
    super::super::manifest::verify_directory(&legacy)?;
    super::super::manifest::verify_published(&published)?;
    Ok(())
}

#[test]
fn corrupt_extra_or_missing_revision_one_inventory_blocks_fence_and_pointer_switch() -> TestResult {
    for mutation in ["tampered", "extra", "missing"] {
        let temporary = tempfile::tempdir()?;
        let store = Store::open(temporary.path().join("store"))?;
        let root = store.out_dir().join("publication");
        let legacy = reconstructed::install(&store, &root)?;
        let pointer = std::fs::read_link(root.join("current"))?;
        match mutation {
            "tampered" => std::fs::write(legacy.join("audit.json"), b"tampered")?,
            "extra" => std::fs::write(legacy.join("event-catalog.json"), b"{}")?,
            "missing" => std::fs::remove_file(legacy.join("recruiting.csv"))?,
            _ => return Err("unknown finite legacy mutation".into()),
        }
        let after_mutation = reconstructed::bytes(&legacy)?;
        check!(super::super::manifest::verify_directory(&legacy).is_err());
        let dataset = ExportDataset::load(&store)?;
        check!(super::super::fence(&root, &dataset).is_err());
        check!(workbook::build(&store, &options()?).is_err());
        check!(eq; std::fs::read_link(root.join("current"))?, pointer);
        check!(eq; reconstructed::bytes(&legacy)?, after_mutation);
    }
    Ok(())
}

#[test]
fn future_manifest_revision_is_dispatched_before_its_shape_is_decoded() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let path = temporary.path().join("manifest.json");
    let retained = br#"{"schema_revision":13,"unknown_new_shape":true}"#;
    std::fs::write(&path, retained)?;
    let error = super::super::manifest::verify_directory(temporary.path())
        .err()
        .ok_or("future manifest")?;
    check!(matches!(
        error,
        crate::report::ReportError::Invariant { .. }
    ));
    check!(eq; std::fs::read(path)?, retained);
    Ok(())
}

#[test]
fn current_frozen_shape_requires_relays_without_a_historical_default() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let store = Store::open(temporary.path().join("store"))?;
    let dataset = ExportDataset::load(&store)?;
    let path = temporary.path().join("missing-relays.json");
    dataset.save_frozen(&path)?;
    let mut frozen: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    frozen
        .get_mut("data")
        .and_then(Value::as_object_mut)
        .ok_or("frozen data")?
        .remove("relays")
        .ok_or("current relays field")?;
    let retained = serde_json::to_vec(&frozen)?;
    std::fs::write(&path, &retained)?;
    let error = ExportDataset::reopen_frozen(&path)
        .err()
        .ok_or("missing required relays")?;
    check!(error.to_string().contains("missing field `relays`"));
    check!(eq; std::fs::read(path)?, retained);
    Ok(())
}
