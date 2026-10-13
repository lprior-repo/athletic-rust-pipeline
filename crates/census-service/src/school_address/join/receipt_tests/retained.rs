use super::*;

#[test]
fn retained_nces_application_pins_real_capture_and_survives_replay_reopen() -> TestResult {
    let dir = tempfile::tempdir()?;
    let retained = fixture::Retained::new(dir.path())?;
    let school = retained.school("Albertville High School")?;
    let store = Store::open(dir.path().join("store"))?;
    store.append(Table::Schools, &school)?;
    let first = retained.replay.apply(&store)?;
    check!(eq; first.counters.linked, 1);
    check!(eq; first.counters.ccd_filled, 1);
    check!(eq; first.counters.review_filed, 0);
    let merged = merged_school(&store)?;
    let claim = merged
        .postal_addresses
        .first()
        .ok_or("real application omitted postal claim")?;
    check!(eq; claim.owner().id.as_str(), "010000500871");
    check!(eq; claim.capture_sha256(), fixture::CCD_SHA);
    check!(eq; claim.source_label(), &SourceLabel::Ccd);
    check!(eq; claim.evidence().source.url.as_deref(), Some(fixture::CCD_URL));
    check!(eq; claim.evidence().observed_on.as_str(), fixture::CAPTURE_DATE);
    let note = claim
        .evidence()
        .note
        .as_deref()
        .ok_or("real claim lacks capture note")?;
    check!(note.contains(&format!("sha256={}", fixture::CCD_SHA)));
    check!(note.contains(&format!(
        "generation {}",
        retained.replay.generation_digest()
    )));
    check!(eq; claim.address().line1().map(StreetLine::as_str), Some("402 E McCord Ave"));
    let witness = witness::single(&store, &school.id, "schools")?;
    assert_real_lane(&witness, &first)?;
    let effect = witness::effect(&witness, Table::Schools)?;
    check!(eq; effect.get("placement"), Some(&json!("unpartitioned")));
    let (store, applied) = physical::checkpoint(store)?;
    check!(eq; applied.schools()?.len(), 2);
    check!(eq; applied.applications()?.len(), 1);
    let again = retained.replay.apply(&store)?;
    check!(eq; again.counters.linked, 0);
    check!(eq; again.counters.already_linked, 1);
    check!(eq; again.counters.backfilled, 0);
    check!(eq; again.counters.postal_filled, 0);
    let (store, after_replay) = physical::checkpoint(store)?;
    check!(eq; after_replay, applied);
    let repeat_after_reopen = retained.replay.apply(&store)?;
    check!(eq; repeat_after_reopen.counters.linked, 0);
    check!(eq; repeat_after_reopen.counters.already_linked, 1);
    let (_store, reopened) = physical::checkpoint(store)?;
    check!(eq; reopened, applied);
    Ok(())
}

fn assert_real_lane(witness: &Value, report: &JoinReport) -> TestResult {
    let lanes = witness
        .get("provenance")
        .and_then(|value| value.get("lanes"))
        .and_then(Value::as_array)
        .ok_or("witness lacks provenance lanes")?;
    check!(eq; lanes.len(), 1);
    let lane = lanes.first().ok_or("real witness has no lane")?;
    check!(eq; lane.get("provider").and_then(Value::as_str), Some("nces-ccd"));
    check!(eq; lane.get("capture_sha256").and_then(Value::as_str), Some(fixture::CCD_SHA));
    check!(eq; lane.get("url").and_then(Value::as_str), Some(fixture::CCD_URL));
    check!(eq; lane.get("observed_on").and_then(Value::as_str), Some(fixture::CAPTURE_DATE));
    let actual = report
        .lanes
        .lanes("nces-ccd")
        .first()
        .ok_or("report lacks real CCD lane")?;
    check!(eq; lane.get("path").and_then(Value::as_str), Some(actual.path.as_str()));
    check!(eq; lane.get("generation").and_then(Value::as_str), Some(actual.generation.as_str()));
    check!(eq; report.lanes.lanes("nces-ccd").len(), 1);
    Ok(())
}

#[test]
fn retained_nces_dry_run_leaves_every_persisted_key_and_value_unchanged() -> TestResult {
    let dir = tempfile::tempdir()?;
    let retained = fixture::Retained::new(dir.path())?;
    let store = Store::open(dir.path().join("store"))?;
    store.append(Table::Schools, &retained.school("Albertville High School")?)?;
    let (store, before) = physical::checkpoint(store)?;
    let report = join_generation(
        &store,
        &retained.generation,
        Some(&dir.path().join("dry-out")),
        retained.replay.overrides().clone(),
        Mode::DryRun,
    )?;
    check!(eq; report.mode.as_str(), "dry-run");
    check!(eq; report.counters.linked, 1);
    check!(eq; report.counters.ccd_filled, 1);
    witness::assert_no_application(&store)?;
    let (_store, after) = physical::checkpoint(store)?;
    check!(eq; after, before);
    Ok(())
}

#[test]
fn a_school_arriving_after_apply_uses_the_same_retained_generation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let retained = fixture::Retained::new(dir.path())?;
    let first_school = retained.school("Albertville High School")?;
    let later_school = retained.school("Hoover High School")?;
    let store = Store::open(dir.path().join("store"))?;
    store.append(Table::Schools, &first_school)?;
    check!(eq; retained.replay.apply(&store)?.counters.linked, 1);
    let first_witnesses = witness::all(&store)?;
    let generation = retained.replay.generation_digest().to_string();
    store.append(Table::Schools, &later_school)?;
    let later = retained.replay.apply(&store)?;
    check!(eq; retained.replay.generation_digest(), generation.as_str());
    check!(eq; later.counters.scanned, 2);
    check!(eq; later.counters.linked, 1);
    check!(eq; later.counters.already_linked, 1);
    check!(eq; later.counters.ccd_filled, 1);
    let witnesses = witness::all(&store)?;
    check!(eq; witnesses.len(), 2);
    check!(first_witnesses
        .iter()
        .all(|value| witnesses.contains(value)));
    let later_witness = witnesses
        .iter()
        .find(|value| value.get("target").and_then(Value::as_str) == Some(later_school.id.as_str()))
        .ok_or("new target has no application witness")?;
    check!(eq; later_witness.get("mutation").and_then(Value::as_str), Some("schools"));
    let (_store, physical) = physical::checkpoint(store)?;
    check!(eq; physical.schools()?.len(), 4);
    check!(eq; physical.applications()?.len(), 2);
    Ok(())
}
