use super::super::super::RESULT_SET_PHASE;
use super::*;
use census_domain::model::{CanonicalEvent, CanonicalMeet, CanonicalTeam, SourceObservation};

mod atomic;
mod cardinality;
mod interruption;

const TABLES: [Table; 6] = [
    Table::Meets,
    Table::Events,
    Table::Teams,
    Table::Athletes,
    Table::Performances,
    Table::SourceObservations,
];

pub(super) fn physical(store: &Store) -> TestResult<Vec<(Table, census_store::TableWalk)>> {
    TABLES
        .into_iter()
        .map(|table| Ok((table, store.walk_table(table)?)))
        .collect()
}

fn assert_absent(store: &Store) -> TestResult {
    for table in TABLES {
        check!(eq; store.walk_table(table)?.rows, 0, "{table:?}");
    }
    check!(store.journal_payloads(RESULT_SET_PHASE)?.is_empty());
    Ok(())
}

pub(super) fn entities(store: &Store) -> TestResult<serde_json::Value> {
    Ok(json!({
        "meets": store.scan::<CanonicalMeet>(Table::Meets)?,
        "events": store.scan::<CanonicalEvent>(Table::Events)?,
        "teams": store.scan::<CanonicalTeam>(Table::Teams)?,
        "athletes": store.scan::<CanonicalAthlete>(Table::Athletes)?,
        "performances": store.scan::<CanonicalPerformance>(Table::Performances)?,
        "observations": store.scan::<SourceObservation>(Table::SourceObservations)?,
    }))
}

fn assert_projected(store: &Store) -> TestResult {
    for (table, count) in [
        (Table::Meets, 1),
        (Table::Events, 3),
        (Table::Teams, 2),
        (Table::Athletes, 2),
        (Table::Performances, 3),
        (Table::SourceObservations, 3),
    ] {
        let walk = store.walk_table(table)?;
        check!(eq; walk.rows, count, "{table:?}");
        if table != Table::SourceObservations {
            check!(eq; walk.repeated_ids, 0, "{table:?}");
        }
    }
    let payloads = store.journal_payloads(RESULT_SET_PHASE)?;
    let receipt = payloads
        .iter()
        .find(|value| value["disposition"] == "projection_applied")
        .ok_or("complete projection")?;
    check!(eq; receipt["projected_rows"], 3);
    check!(eq; receipt["meet"], "725218");
    check!(eq; receipt["rsid"], "1266814");
    check!(eq; receipt["capture"]["content_digest"], crate::net::cache::content_digest(TROY));
    check!(eq; receipt["raw_metadata_capture"]["content_digest"], crate::net::cache::content_digest(FEMALE_RAW));
    check!(eq; receipt["source_completeness"], "unknown");
    check!(eq; receipt["ownership_complete"], false);
    check!(eq; receipt["canonical_identity_accepted"], false);
    check!(eq; receipt["lifetime_pr_claimed"], false);
    check!(eq; receipt["census_sealed"], false);
    let state = entities(store)?;
    check!(eq; state["meets"][0]["date"], "2026-03-27");
    let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
    check!(eq;
        performances.iter().map(|row| row.source_key.as_str()).collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from(["milesplit_result:201782263", "milesplit_result:201782277", "milesplit_result:201782806"])
    );
    check!(performances
        .iter()
        .all(|row| row.observed_grade.is_none() && row.date == "2026-03-27"));
    let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
    check!(eq;
        athletes.iter().map(|row| Ok(row.source.as_ref().ok_or("owner")?.id.as_str()))
            .collect::<TestResult<std::collections::BTreeSet<_>>>()?,
        std::collections::BTreeSet::from(["14222592", "11357806"])
    );
    Ok(())
}
