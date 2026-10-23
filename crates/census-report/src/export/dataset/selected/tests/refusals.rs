use super::{ExportDataset, Fixture, Table, TestResult, Value};
use crate::export::dataset::selected_proofs::SelectedProofs;
use census_domain::model::{ProfileSelectionV1, StoreIdentity};
use sha2::{Digest, Sha256};

#[tokio::test]
async fn selected_reopen_requires_the_exact_external_archive_commitment() -> TestResult {
    let fixture = Fixture::new().await?;
    let objects = super::temporary()?;
    let archive = super::temporary()?;
    let dataset = fixture.capture(objects.path())?;
    let path = archive.path().join("selected.json");
    dataset.save_frozen(&path)?;
    check!(ExportDataset::reopen_frozen(&path).is_err());
    for changed in [
        String::new(),
        "0".repeat(64),
        dataset.lineage.input_digest.clone(),
    ] {
        check!(ne; changed, dataset.archive_digest()?);
        check!(ExportDataset::reopen_frozen_with_digest(&path, &changed).is_err());
    }
    let original = std::fs::read(&path)?;
    let mut wire: Value = serde_json::from_slice(&original)?;
    wire.get_mut("selected_proofs")
        .and_then(Value::as_object_mut)
        .ok_or("proof envelope")?
        .insert("participation_digest".into(), Value::String("0".repeat(64)));
    std::fs::write(&path, serde_json::to_vec(&wire)?)?;
    check!(ExportDataset::reopen_frozen_with_digest(&path, &dataset.archive_digest()?).is_err());
    check!(eq; std::fs::read(&path)?, serde_json::to_vec(&wire)?);
    Ok(())
}

#[tokio::test]
async fn all_independent_physical_roots_are_enforced_on_capture_and_replay() -> TestResult {
    let fixture = Fixture::new().await?;
    let objects = super::temporary()?;
    let mut dataset = fixture.capture(objects.path())?;
    let profile = dataset.selected_authority()?.profile().clone();
    let fingerprints = dataset.selected_authority()?.fingerprints().clone();
    let proofs = dataset.selected_proofs.take().ok_or("selected proofs")?;
    let original = census_domain::model::serialized_digest(&proofs)?;
    for table in [
        Table::Athletes,
        Table::Performances,
        Table::Teams,
        Table::Meets,
        Table::Events,
    ] {
        let mut roots = fingerprints.clone();
        let original = roots
            .insert(table.file().into(), "0".repeat(64))
            .ok_or("physical root")?;
        check!(ne; original, "0".repeat(64));
        let fresh = super::temporary()?;
        check!(SelectedProofs::capture(
            &mut dataset,
            &fixture.store.snapshot(),
            &profile,
            &roots,
            fresh.path(),
            fixture
                .store
                .snapshot()
                .capture_frozen_public_identity_v2()?,
            None,
        )
        .is_err());
        check!(std::fs::read_dir(fresh.path())?.next().is_none());
        check!(proofs
            .hydrate(&mut dataset, &profile, &roots, objects.path())
            .is_err());
        roots.remove(table.file()).ok_or("omitted root")?;
        check!(proofs
            .hydrate(&mut dataset, &profile, &roots, objects.path())
            .is_err());
    }
    check!(eq; census_domain::model::serialized_digest(&proofs)?, original);
    dataset.selected_proofs = Some(proofs);
    dataset.verified_participation()?;
    dataset.verified_current_affiliations()?;
    dataset.verified_public_identities()?;
    Ok(())
}

#[tokio::test]
async fn foreign_and_stale_captured_identity_refuse_before_writing_chunks() -> TestResult {
    let fixture = Fixture::new().await?;
    let stale = fixture
        .store
        .snapshot()
        .capture_frozen_public_identity_v2()?;
    fixture.append_second_meet().await?;
    let foreign = Fixture::new().await?;
    let foreign = foreign
        .store
        .snapshot()
        .capture_frozen_public_identity_v2()?;
    let objects = super::temporary()?;
    let mut dataset = fixture.capture(objects.path())?;
    let profile = dataset.selected_authority()?.profile().clone();
    let fingerprints = dataset.selected_authority()?.fingerprints().clone();
    let unchanged = dataset.archive_digest()?;
    for (captured, expected) in [
        (foreign, "selected public identity authority changed"),
        (
            stale,
            "public identity view differs from the exact dataset lineage or registry head",
        ),
    ] {
        let fresh = super::temporary()?;
        let result = SelectedProofs::capture(
            &mut dataset,
            &fixture.store.snapshot(),
            &profile,
            &fingerprints,
            fresh.path(),
            captured,
            None,
        );
        let detail = result
            .err()
            .ok_or("captured identity unexpectedly accepted")?
            .to_string();
        if !detail.contains(expected) {
            return Err(format!("unexpected captured identity refusal: {detail}").into());
        }
        check!(std::fs::read_dir(fresh.path())?.next().is_none());
        check!(eq; dataset.archive_digest()?, unchanged);
    }
    Ok(())
}

#[tokio::test]
async fn missing_envelope_and_objects_cannot_restore_selected_verification() -> TestResult {
    let fixture = Fixture::new().await?;
    let objects = super::temporary()?;
    let archive = super::temporary()?;
    let dataset = fixture.capture(objects.path())?;
    let path = archive.path().join("selected.json");
    dataset.save_frozen(&path)?;
    let digest = dataset.archive_digest()?;
    let mut wire: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    wire.as_object_mut()
        .ok_or("archive object")?
        .remove("selected_proofs")
        .ok_or("proof envelope")?;
    let missing = archive.path().join("missing.json");
    let missing_bytes = serde_json::to_vec(&wire)?;
    std::fs::write(&missing, &missing_bytes)?;
    check!(ExportDataset::reopen_frozen_with_digest(&missing, &digest).is_err());
    let proofless_digest = format!("{:x}", Sha256::digest(&missing_bytes));
    let proofless = ExportDataset::reopen_frozen_with_digest(&missing, &proofless_digest)?;
    check!(proofless.verified_participation().is_err());
    check!(proofless.verified_current_affiliations().is_err());
    check!(proofless.verified_public_identities().is_err());
    let proof_directory = super::super::super::frozen::proof_directory(&path)?;
    let object = std::fs::read_dir(&proof_directory)?
        .next()
        .ok_or("saved object")??
        .path();
    let retained = std::fs::read(&object)?;
    std::fs::remove_file(&object)?;
    check!(ExportDataset::reopen_frozen_with_digest(&path, &digest).is_err());
    std::fs::write(&object, retained)?;
    std::fs::write(
        proof_directory.join("0".repeat(64)),
        b"unlisted retained bytes",
    )?;
    check!(ExportDataset::reopen_frozen_with_digest(&path, &digest).is_err());
    Ok(())
}

#[tokio::test]
async fn selected_capture_refuses_foreign_selection_snapshot_and_nonempty_objects() -> TestResult {
    let fixture = Fixture::new().await?;
    let objects = super::temporary()?;
    let foreign = ProfileSelectionV1::new(
        StoreIdentity::parse(&"0".repeat(64))?,
        super::fixture::profile()?.id()?,
    );
    check!(ExportDataset::from_selected_snapshot(
        &fixture.store,
        &fixture.store.snapshot(),
        &foreign,
        objects.path()
    )
    .is_err());
    let other_directory = super::temporary()?;
    let other = census_store::Store::open(other_directory.path().join("store"))?;
    check!(ExportDataset::from_selected_snapshot(
        &fixture.store,
        &other.snapshot(),
        &fixture.selection,
        objects.path()
    )
    .is_err());
    let marker = objects.path().join("caller-owned-marker");
    std::fs::write(&marker, b"retain me")?;
    check!(fixture.capture(objects.path()).is_err());
    check!(eq; std::fs::read(&marker)?, b"retain me");
    Ok(())
}

#[tokio::test]
async fn graph_identity_refuses_generic_dataset_without_disabling_selected_capture() -> TestResult {
    let fixture = Fixture::composed(true).await?;
    let snapshot = fixture.store.snapshot();
    let sequence = snapshot.sequence();
    let physical = snapshot.tables_digest(&Table::ALL)?;
    for _attempt in 0..2 {
        check!(matches!(
            ExportDataset::from_snapshot(&fixture.store, &snapshot),
            Err(crate::report::ReportError::Store(
                census_store::StoreError::PublicIdentity(
                    census_store::PublicIdentityError::OptInRequired
                )
            ))
        ));
        check!(eq; fixture.store.snapshot().sequence(), sequence);
        check!(eq; fixture.store.snapshot().tables_digest(&Table::ALL)?, physical);
        check!(matches!(
            ExportDataset::load(&fixture.store),
            Err(crate::report::ReportError::Store(
                census_store::StoreError::PublicIdentity(
                    census_store::PublicIdentityError::OptInRequired
                )
            ))
        ));
        check!(eq; fixture.store.snapshot().sequence(), sequence);
        check!(eq; fixture.store.snapshot().tables_digest(&Table::ALL)?, physical);
    }
    let objects = super::temporary()?;
    let selected = fixture.capture(objects.path())?;
    selected.ensure_current_selected_write()?;
    check!(selected.selected_profile_id().is_some());
    selected.verified_participation()?;
    selected.verified_current_affiliations()?;
    check!(selected
        .verified_public_identities()?
        .public_id(fixture.roster_athlete.as_str())
        .is_some());
    check!(eq; fixture.store.snapshot().sequence(), sequence);
    check!(eq; fixture.store.snapshot().tables_digest(&Table::ALL)?, physical);
    Ok(())
}
