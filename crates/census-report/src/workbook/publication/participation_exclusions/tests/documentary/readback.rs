use super::*;

#[test]
fn documentary_readback_rejects_each_changed_field_at_identical_shape_and_count() -> TestResult {
    let fixture = Fixture::empty()?;
    let view = view(&fixture, rows()?)?;
    let outcome = outcome(&fixture, &view)?;
    let directory = tempfile::tempdir()?;
    write(directory.path(), &fixture.dataset, &view, &outcome)?;
    let original = super::super::readback::read(directory.path())?;
    for field in 0..8 {
        let mut changed = original.clone();
        changed
            .get_mut(1)
            .and_then(|record| record.get_mut(field))
            .ok_or("first record field")?
            .push_str("changed");
        super::super::readback::save(directory.path(), &changed)?;
        check!(verify(directory.path(), &fixture.dataset, &view, &outcome).is_err());
    }
    let eligible = original
        .iter()
        .position(|record| {
            record.get(5).is_some_and(|value| {
                value == "[\"documentary_assessment_eligible\",\"school_binding_unresolved\"]"
            })
        })
        .ok_or("eligible documentary row")?;
    for (column, replacement) in [
        (3, "excluded"),
        (4, "[\"missing_performance_date\"]"),
        (5, "[\"documentary_assessment_unresolved\",\"school_binding_unresolved\"]"),
        (5, "[\"documentary_assessment_eligible\"]"),
        (5, "[\"documentary_assessment_eligible\",\"documentary_assessment_unresolved\",\"school_binding_unresolved\"]"),
        (6, "[\"foreign-capture\"]"),
    ] {
        let mut changed = original.clone();
        *changed.get_mut(eligible).and_then(|record| record.get_mut(column)).ok_or("eligible field")? = replacement.into();
        super::super::readback::save(directory.path(), &changed)?;
        check!(verify(directory.path(), &fixture.dataset, &view, &outcome).is_err());
    }
    super::super::readback::save(directory.path(), &original)?;
    check!(eq; verify(directory.path(), &fixture.dataset, &view, &outcome)?, 3);
    Ok(())
}

#[test]
fn documentary_readback_rejects_dropped_duplicate_and_displaced_identity_rows() -> TestResult {
    let fixture = Fixture::empty()?;
    let view = view(&fixture, rows()?)?;
    let outcome = outcome(&fixture, &view)?;
    let directory = tempfile::tempdir()?;
    write(directory.path(), &fixture.dataset, &view, &outcome)?;
    let original = super::super::readback::read(directory.path())?;
    let mut missing = original.clone();
    missing.pop().ok_or("documentary record")?;
    let mut duplicate = original.clone();
    *duplicate.get_mut(2).ok_or("second record")? = original.get(1).ok_or("first record")?.clone();
    let mut displaced = original.clone();
    let candidate = original
        .get(2)
        .and_then(|row| row.first())
        .ok_or("second candidate")?
        .clone();
    *displaced
        .get_mut(1)
        .and_then(|row| row.first_mut())
        .ok_or("first candidate")? = candidate;
    for changed in [missing, duplicate, displaced] {
        super::super::readback::save(directory.path(), &changed)?;
        check!(verify(directory.path(), &fixture.dataset, &view, &outcome).is_err());
    }
    Ok(())
}
