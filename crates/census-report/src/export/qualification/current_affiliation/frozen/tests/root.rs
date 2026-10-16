use super::*;

#[test]
fn self_rehashed_archive_replacement_cannot_change_original_frozen_unknown() -> TestResult {
    let fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", true)?;
    let original_body = std::fs::read(&fixture.body_path)?;
    std::fs::remove_file(&fixture.body_path)?;
    let directory = tempfile::tempdir()?;
    let objects = objects(&directory)?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let expected_table = snapshot.tables_digest(&[Table::Athletes])?;
    let (original_descriptor, proof) = capture(&dataset, &snapshot, &objects)?;
    let original_root = original_descriptor.digest()?;
    check!(eq; proof.into_verified().resolution(&fixture.athlete), &super::super::super::CurrentAffiliationResolution::Unknown);
    std::fs::write(&fixture.body_path, &original_body)?;
    let (available_descriptor, available_proof) = capture(&dataset, &snapshot, &objects)?;
    check!(eq; available_proof.into_verified().resolution(&fixture.athlete).school(), Some(&fixture.school));
    check!(eq; available_descriptor.observations(), original_descriptor.observations());
    check!(ne; available_descriptor.digest()?, original_root);
    let mut forged = original_descriptor.clone();
    forged.archives = available_descriptor.archives;
    root_refused(replay(
        &dataset,
        &objects,
        &forged,
        &expected_table,
        &original_root,
    ))?;
    let mut callbacks = 0;
    root_refused(visit_athletes(
        &dataset,
        &objects,
        &forged,
        &expected_table,
        &original_root,
        |_| {
            callbacks += 1;
            Ok(())
        },
    ))?;
    check!(eq; callbacks, 0);
    let replayed = replay(
        &dataset,
        &objects,
        &original_descriptor,
        &expected_table,
        &original_root,
    )?
    .into_verified();
    check!(eq; replayed.resolution(&fixture.athlete), &super::super::super::CurrentAffiliationResolution::Unknown);
    check!(eq; replayed.audit(&fixture.athlete).ok_or("retained unknown audit")?.unavailable(), &["archive_unavailable".to_string()]);
    Ok(())
}

#[test]
fn frozen_descriptor_root_is_required_before_any_raw_row_callback() -> TestResult {
    let fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", true)?;
    let directory = tempfile::tempdir()?;
    let objects = objects(&directory)?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let expected_table = snapshot.tables_digest(&[Table::Athletes])?;
    let (descriptor, _) = capture(&dataset, &snapshot, &objects)?;
    let absent_objects = directory.path().join("absent-objects");
    for root in [String::new(), "A".repeat(64), "a".repeat(64)] {
        let mut callbacks = 0;
        root_refused(visit_athletes(
            &dataset,
            &absent_objects,
            &descriptor,
            &expected_table,
            &root,
            |_| {
                callbacks += 1;
                Ok(())
            },
        ))?;
        check!(eq; callbacks, 0);
        root_refused(replay(
            &dataset,
            &absent_objects,
            &descriptor,
            &expected_table,
            &root,
        ))?;
    }
    let mut callbacks = 0;
    visit_athletes(
        &dataset,
        &objects,
        &descriptor,
        &expected_table,
        &descriptor.digest()?,
        |_| {
            callbacks += 1;
            Ok(())
        },
    )?;
    check!(eq; callbacks, 1);
    Ok(())
}

fn root_refused<T>(result: ReportResult<T>) -> TestResult {
    match result {
        Err(crate::report::ReportError::Store(StoreError::Refused { detail })) => {
            check!(eq; detail, "frozen affiliation descriptor differs from its externally bound root");
            Ok(())
        }
        Err(error) => Err(error.into()),
        Ok(_) => Err("unbound frozen affiliation descriptor was accepted".into()),
    }
}
