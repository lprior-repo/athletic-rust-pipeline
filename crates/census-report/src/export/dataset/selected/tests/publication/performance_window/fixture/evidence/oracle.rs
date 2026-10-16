use super::{CanonicalPerformance, TestResult};
use census_crawl::milesplit::{
    read_retained_owned_result, CaptureReadBudget, OwnedAcquisitionRef, OWNED_CAPTURE_PHASE,
};
use census_domain::model::{
    serialized_digest, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalTeam, Evidence,
};
use census_store::{ReadBudget, Store, StoreResult, Table};
use serde_json::Value;
use std::collections::BTreeSet;

const BUDGET: ReadBudget = ReadBudget {
    rows: 64,
    bytes: 1024 * 1024,
    record_bytes: 64 * 1024,
};

pub(super) fn references(
    store: &Store,
    performance: &CanonicalPerformance,
    mut refs: BTreeSet<String>,
) -> TestResult<Vec<String>> {
    let snapshot = store.snapshot();
    check!(eq; snapshot.athletes()?.len(), 1);
    check!(eq; snapshot.scan::<CanonicalPerformance>(Table::Performances)?.len(), 2);
    check!(eq; snapshot.scan::<CanonicalTeam>(Table::Teams)?.len(), 1);
    check!(eq; snapshot.scan::<CanonicalEvent>(Table::Events)?.len(), 1);
    check!(eq; snapshot.scan::<CanonicalMeet>(Table::Meets)?.len(), 2);
    snapshot.for_each_observation_bounded::<CanonicalAthlete>(
        Table::Athletes,
        BUDGET,
        |row, _| canonical(&mut refs, &row.evidence),
    )?;
    snapshot.for_each_observation_bounded::<CanonicalTeam>(Table::Teams, BUDGET, |row, _| {
        if row.id == performance.team {
            canonical(&mut refs, &row.evidence)
        } else {
            Ok(())
        }
    })?;
    snapshot.for_each_observation_bounded::<CanonicalEvent>(Table::Events, BUDGET, |row, _| {
        if row.id == performance.event {
            canonical(&mut refs, &row.evidence)
        } else {
            Ok(())
        }
    })?;
    snapshot.for_each_observation_bounded::<CanonicalMeet>(Table::Meets, BUDGET, |row, _| {
        if row.id == performance.meet {
            canonical(&mut refs, &row.evidence)
        } else {
            Ok(())
        }
    })?;
    snapshot.for_each_observation_bounded::<CanonicalPerformance>(
        Table::Performances,
        BUDGET,
        |row, _| {
            if row.id == performance.id {
                canonical(&mut refs, &row.evidence)
            } else {
                Ok(())
            }
        },
    )?;
    let meta = performance
        .evidence
        .iter()
        .filter_map(|evidence| evidence.note.as_deref())
        .map(serde_json::from_str::<Value>)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .find(|note| {
            note.get("role").and_then(Value::as_str)
                == Some("raw result file meet/season metadata only")
        })
        .ok_or("authored raw metadata note")?;
    let meet = meta
        .get("meet_id")
        .and_then(Value::as_str)
        .ok_or("authored meet")?;
    let prefix = format!("acquisition/{meet}/");
    let keys: Vec<_> = store
        .journal_keys(OWNED_CAPTURE_PHASE)?
        .into_iter()
        .filter(|key| key.starts_with(&prefix) && key.ends_with("/manifest"))
        .collect();
    check!(eq; keys.len(), 1);
    let acquisition = OwnedAcquisitionRef::parse(keys.first().ok_or("authored acquisition")?)?;
    let source = read_retained_owned_result(
        &snapshot,
        &acquisition,
        "data[0]",
        CaptureReadBudget::new(1024 * 1024)?,
    )?;
    check!(eq; source.row().source_athlete.id, "14222592");
    refs.insert(acquisition.manifest_key().to_owned());
    refs.insert(source.interpretation_key().to_owned());
    refs.insert(
        meta.pointer("/raw_metadata_capture/archive_manifest")
            .and_then(Value::as_str)
            .ok_or("authored metadata reference")?
            .to_owned(),
    );
    Ok(refs.into_iter().collect())
}

fn canonical(refs: &mut BTreeSet<String>, evidence: &[Evidence]) -> StoreResult<()> {
    for item in evidence {
        refs.insert(format!("canonical_evidence:{}", serialized_digest(item)?));
    }
    Ok(())
}
