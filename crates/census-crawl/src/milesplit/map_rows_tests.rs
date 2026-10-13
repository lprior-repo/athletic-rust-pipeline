use super::*;
use crate::milesplit::owned::{parse_owned_meet, OwnedMeetVerdict};
use crate::milesplit::results::{Accumulator, Stats};
use census_domain::model::{CentiMetres, SourceIdentity, SourceNamespace};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn same_identity_milesplit_rows_preserve_distinct_marks_and_source_observations() -> TestResult {
    let body = include_bytes!("owned/fixtures/troy_725218.json");
    let OwnedMeetVerdict::Parsed(page) = parse_owned_meet(body, 725218) else {
        return Err("parsed owned fixture".into());
    };
    let first = page.rows.first().ok_or("individual fixture row")?;
    let mut second = first.clone();
    second.mark = census_domain::model::Mark::DistanceMetres(CentiMetres::new(500));
    second.provider["teamName"] = serde_json::json!("Alternate published school spelling");
    second.provider["mark"] = serde_json::json!("5.00m");
    let raw = crate::milesplit::raw::parse_raw(
        include_str!("../../tests/fixtures/milesplit/troy_725218_rs1266814_raw_projection.html"),
        "https://al.milesplit.com/meets/725218/results/1266814/raw",
    )?;
    let reference = crate::milesplit::wire::ResultSetRef::parse(
        "https://al.milesplit.com/meets/725218/results/1266814/raw",
    )
    .ok_or("result-set reference")?;
    let capture = crate::net::FetchOutcome {
        url: "https://al.milesplit.com/api/v1/meets/725218/performances".into(),
        content_digest: crate::net::cache::content_digest(body),
        fetched_at: "2026-10-01T23:44:16Z".into(),
        response_url: None,
        method: "GET".into(),
        status: 200,
        bytes: body.len(),
        from_cache: true,
        content_type: Some("application/json".into()),
        body: body.to_vec(),
    };
    let source = SourceContext {
        page: &raw,
        reference: &reference,
        capture: &capture,
        performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 10, 1).ok_or("cutoff")?,
    };
    let (meet, metadata) = super::super::meet_for(&source)?;
    let context = MeetContext {
        meet: &meet,
        metadata: &metadata,
        source: &source,
    };
    let (mut school, _) = census_domain::model::CanonicalSchool::new(
        census_domain::UsJurisdiction::Alabama,
        "Provider school",
        "provider",
        None,
    );
    school.source_identities.push(SourceIdentity::new(
        SourceNamespace::MilesplitSchool,
        first.team_id.to_string(),
    ));
    let schools = super::super::ProviderSchools::from_schools(&[school]);
    let mut stats = Stats::default();
    let mut accumulated = Accumulator::default();
    let mut writer = RowWriter {
        schools: &schools,
        stats: &mut stats,
        accumulated: &mut accumulated,
    };
    for row in [first, &second, first, &second] {
        check!(eq; record_row(&mut writer, &context, row)?, 1);
    }
    check!(eq; accumulated.performances.len(), 2);
    check!(eq; accumulated.observations.len(), 2);
    let originals: Vec<_> = accumulated.performances.into_values().collect();
    check!(eq; originals.first().ok_or("first performance")?.id,
        originals.last().ok_or("second performance")?.id);
    check!(originals.iter().any(|row| row.mark == first.mark));
    check!(originals.iter().any(|row| row.mark == second.mark));
    check!(originals
        .iter()
        .all(|row| row.source_athlete.as_ref() == Some(&first.source_athlete)));
    let observations: Vec<_> = accumulated.observations.into_values().collect();
    check!(eq; observations.first().ok_or("first source row")?.id(),
        observations.last().ok_or("second source row")?.id());
    let directory = tempfile::tempdir()?;
    let store = census_store::Store::open(directory.path())?;
    store.append_many(census_store::Table::Performances, &originals)?;
    store.append_many(census_store::Table::SourceObservations, &observations)?;
    let rows: Vec<CanonicalPerformance> = store.scan(census_store::Table::Performances)?;
    check!(eq; rows.len(), 1);
    check!(eq; rows.first().ok_or("canonical performance")?.retained_conflicts.len(), 1);
    let mut physical = Vec::new();
    store.snapshot().for_each_observation(
        census_store::Table::Performances,
        |row: CanonicalPerformance| {
            physical.push(row);
            Ok(())
        },
    )?;
    check!(eq; physical.len(), 2);
    check!(originals.iter().all(|row| physical.contains(row)));
    let mut physical_sources = Vec::new();
    store.snapshot().for_each_observation(
        census_store::Table::SourceObservations,
        |row: census_domain::model::SourceObservation| {
            physical_sources.push(row);
            Ok(())
        },
    )?;
    check!(eq; physical_sources.len(), 2);
    check!(observations
        .iter()
        .all(|row| physical_sources.contains(row)));
    Ok(())
}
