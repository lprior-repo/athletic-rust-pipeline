use super::*;

mod readback;
mod setup;
use setup::Transfer;

#[derive(Clone, Copy)]
enum Case {
    Current,
    Conflict,
    Unknown,
}

#[test]
fn controlled_reviewed_transfer_preserves_old_school_performance_and_current_postal_geography_offline(
) -> TestResult {
    for case in [Case::Current, Case::Conflict, Case::Unknown] {
        exercise(case)?;
    }
    Ok(())
}

fn exercise(case: Case) -> TestResult {
    let transfer = setup::prepare(case).map_err(|error| format!("transfer setup: {error}"))?;
    let objects = tempfile::tempdir()?;
    let archive = tempfile::tempdir()?;
    let output = tempfile::tempdir()?;
    let dataset = capture(&transfer.fixture, objects.path())
        .map_err(|error| format!("transfer selected capture: {error}"))?;
    let before = inspect(&dataset, &transfer)?;
    let frozen = archive.path().join("controlled-transfer.json");
    let digest = dataset.archive_digest()?;
    let lineage = dataset.lineage.clone();
    dataset
        .save_frozen(&frozen)
        .map_err(|error| format!("transfer frozen save: {error}"))?;
    let wire = inspect_wire(&frozen)?;
    inspect_family(&frozen, &wire, &transfer)?;
    let workbook = publish(
        &transfer.fixture,
        &dataset,
        &output.path().join("publication"),
    )
    .map_err(|error| format!("transfer publication: {error}"))?;
    let rows = readback::artifacts(&workbook, &dataset, &transfer)?;
    let published_digest = published::verify_published(&workbook)
        .map_err(|error| format!("transfer artifact verification: {error}"))?
        .generation_digest;
    drop(dataset);
    let Transfer {
        fixture,
        old,
        current,
        address_digest,
        case,
    } = transfer;
    remove_sources(fixture, objects)?;
    let expected = Retained {
        old,
        current,
        address_digest,
        case,
    };
    let reopened = ExportDataset::reopen_frozen_with_digest(&frozen, &digest)
        .map_err(|error| format!("transfer independent reopen: {error}"))?;
    check!(eq; inspect_retained(&reopened, &expected)?, before);
    let publication =
        published::open_publication(&workbook, &lineage, Scope::AllSources, GradYear::CO2027)
            .map_err(|error| format!("transfer publication reopen: {error}"))?;
    check!(eq; inspect_retained(&publication.dataset, &expected)?, before);
    check!(eq; readback::retained_artifacts(&workbook, &publication.dataset, &expected)?, rows);
    check!(eq; published::verify_published(&workbook)?.generation_digest, published_digest);
    Ok(())
}

struct Retained {
    old: fixture::Capture,
    current: fixture::Capture,
    address_digest: String,
    case: Case,
}

fn inspect(dataset: &ExportDataset, transfer: &Transfer) -> TestResult<Value> {
    inspect_parts(
        dataset,
        &transfer.old,
        &transfer.current,
        &transfer.address_digest,
        transfer.case,
    )
}
fn inspect_retained(dataset: &ExportDataset, retained: &Retained) -> TestResult<Value> {
    inspect_parts(
        dataset,
        &retained.old,
        &retained.current,
        &retained.address_digest,
        retained.case,
    )
}
fn inspect_parts(
    dataset: &ExportDataset,
    old: &fixture::Capture,
    current: &fixture::Capture,
    address: &str,
    case: Case,
) -> TestResult<Value> {
    check!(ne; old.school.id, current.school.id);
    check!(ne; old.athlete.id, current.athlete.id);
    check!(eq; dataset.performances.len(), 1);
    let performance = dataset
        .performances
        .first()
        .ok_or("required historical performance")?;
    check!(eq; performance.athlete, old.athlete.id);
    check!(eq; dataset.teams.get(&performance.team).ok_or("historical performance team")?.school, old.school.id);
    check!(eq; dataset.schools.get(&old.school.id).ok_or("historical school")?.state, Some(UsJurisdiction::Wisconsin));
    check!(eq; dataset.schools.get(&current.school.id).ok_or("current school")?.state, Some(UsJurisdiction::Michigan));
    let identities = dataset.verified_public_identities()?;
    let old_public = identities
        .public_id(old.athlete.id.as_str())
        .ok_or("accepted original occurrence")?;
    check!(eq; identities.public_id(current.athlete.id.as_str()), Some(old_public));
    check!(eq; identities.group(current.athlete.id.as_str()).ok_or("accepted current occurrence")?.canonical_candidate().as_str(), old.athlete.id.as_str());
    let affiliations = dataset.verified_current_affiliations()?;
    let resolution = affiliations.resolution(&old.athlete.id);
    let expected_status = match case {
        Case::Current => "resolved_at_selected_cutoff",
        Case::Conflict => "conflicting_affiliation_evidence",
        Case::Unknown => "current_affiliation_unknown",
    };
    check!(eq; resolution.status(), expected_status);
    check!(eq; resolution.school(), matches!(case, Case::Current).then_some(&current.school.id));
    for (state, count) in [
        (
            UsJurisdiction::Michigan,
            usize::from(matches!(case, Case::Current)),
        ),
        (UsJurisdiction::Wisconsin, 0),
    ] {
        let selected = crate::report::Derivation::selected(
            dataset,
            Scope::AllSources,
            Some(2027),
            &[state],
            affiliations,
        )?;
        check!(eq; selected.athletes().len(), count);
        if count == 1 {
            check!(eq; selected.athletes().first().ok_or("current selected transfer")?.school, current.school.id);
        }
    }
    let participation = dataset
        .verified_participation()?
        .assessment(&performance.id)
        .ok_or("historical participation audit")?;
    check!(eq; participation.outcome(), crate::export::ParticipationOutcome::Unresolved);
    let audit = affiliations
        .audit(&old.athlete.id)
        .ok_or("retained transfer affiliation audit")?;
    check!(!audit.references().is_empty());
    if !matches!(case, Case::Conflict) {
        check!(audit.references().iter().any(|reference| reference
            .starts_with("roster_selector:OutdoorTrack:")
            && reference.ends_with(":OutsideEffectiveScope")));
    }
    let selected_school = dataset
        .schools
        .get(&current.school.id)
        .ok_or("current postal school")?;
    check!(serde_json::to_string(selected_school)?.contains(address));
    for original in [old, current] {
        let physical = dataset
            .athletes
            .iter()
            .find(|athlete| athlete.id == original.athlete.id)
            .ok_or("original candidate retained")?;
        check!(eq; physical.school, original.school.id);
        check!(eq; physical.evidence, original.athlete.evidence);
        check!(eq; physical.source, original.athlete.source);
    }
    Ok(
        json!({"data":dataset,"public":old_public,"identity_head":identities.head(),"status":resolution.status(),"references":audit.references(),"gaps":audit.unavailable(),"participation":format!("{:?}",participation.outcome())}),
    )
}

fn inspect_family(frozen: &Path, wire: &Value, transfer: &Transfer) -> TestResult {
    let digest = wire
        .pointer("/selected_proofs/affiliation/roster_family/digest")
        .and_then(Value::as_str)
        .ok_or("reviewed family object")?;
    let directory = frozen.with_file_name("controlled-transfer.json.proofs");
    let family: Value = serde_json::from_slice(&std::fs::read(directory.join(digest))?)?;
    check!(eq; family.get(1).and_then(Value::as_array).ok_or("complete transfer captures")?.len(), 2);
    let closure_digest = wire
        .pointer("/selected_proofs/affiliation/archives/digest")
        .and_then(Value::as_str)
        .ok_or("reviewed closure")?;
    let closure: Value = serde_json::from_slice(&std::fs::read(directory.join(closure_digest))?)?;
    let reviewed_digest = closure
        .pointer("/reviewed/digest")
        .and_then(Value::as_str)
        .ok_or("reviewed source bundle")?;
    let reviewed = std::fs::read(directory.join(reviewed_digest))?;
    let reference: census_store::RosterFrozenReviewedClaimsReferenceV1 =
        serde_json::from_value(closure.get("reviewed").ok_or("source reference")?.clone())?;
    let mut grant = AttachmentReplayGrantV1::new();
    let replay =
        census_store::replay_frozen_roster_reviewed_claims_v1(&reviewed, &reference, &mut grant)
            .map_err(|error| {
                format!("transfer family replay ({} bytes): {error}", reviewed.len())
            })?;
    let claims = replay.claims(&mut grant)?;
    check!(eq; claims.len(), 2);
    let mut days = std::collections::BTreeSet::new();
    for claim in claims {
        days.insert(
            claim
                .proposal(&mut grant)?
                .capture()
                .observed_day()
                .as_str()
                .to_owned(),
        );
    }
    let old_day = transfer
        .old
        .packet
        .roster()
        .acquired_at()
        .date_naive()
        .to_string();
    check!(days.contains(&old_day));
    check!(days.contains("2026-10-16"));
    Ok(())
}
