use super::*;
use census_crawl::result_file::ParsedMeet;
use census_domain::model::{CompetitionLevel, Evidence, SourceRef};
use std::path::Path;

pub(super) fn source_report(raw: &str, date: &str) -> TestResult<ParsedMeet> {
    let lines = vec![
        format!("Exact Source Meet - {date}"),
        "Boys 100 Meter Dash Finals".to_string(),
        format!(
            "    {:<20} {:>4} {:<20} {:>24}",
            "Name", "Year", "School", "Finals"
        ),
        format!(
            "  1 {:<20} {:>4} {:<20} {:>24}",
            "Synthetic runner", 11, "Synthetic school", raw
        ),
    ];
    census_crawl::hytek::parse(
        &lines,
        SourceRef::new(
            "wiaa_results",
            Some("https://example.test/exact-time".into()),
        ),
    )
    .ok_or_else(|| "source report did not parse".into())
}

pub(super) fn store_with_source_reports(
    root: &Path,
    reports: &[(&str, &str)],
) -> TestResult<Store> {
    let seed = selection_dataset(EventKind::Track100m)?;
    let store = Store::open(root)?;
    for school in seed.schools.values() {
        store.append(Table::Schools, school)?;
    }
    store.append_many(Table::Athletes, &seed.athletes)?;
    for (raw, date) in reports {
        let parsed = source_report(raw, date)?;
        check!(eq; (parsed.rows_parsed, parsed.rows_skipped), (1, 0), "{raw}");
        let event = parsed
            .events
            .into_iter()
            .next()
            .ok_or("missing parsed event")?;
        let row = event.rows.into_iter().next().ok_or("missing parsed row")?;
        let mut meet = CanonicalMeet::new_checked(
            Some(UsJurisdiction::Wisconsin),
            parsed.name,
            parsed.date,
            CompetitionLevel::Invitational,
        )?;
        meet.sports = vec![Sport::OutdoorTrack];
        let specification = EventSpecification::from_published_label(&event.label, &event.kind)?;
        let canonical_event = CanonicalEvent::new(
            EventIdentity {
                meet: &meet.id,
                kind: event.kind,
                gender: event.gender,
                division: event.division.as_deref(),
                round: event.round.as_deref(),
            },
            specification,
        )?;
        let mut performance = reported(&seed, &format!("source:{date}:{raw}"), row.mark);
        performance.meet = meet.id.clone();
        performance.event = canonical_event.id.clone();
        performance.date = meet.date.clone();
        performance.observed_grade = row.grade;
        performance.evidence = vec![Evidence::parsed(
            SourceRef::new(
                "wiaa_results",
                Some("https://example.test/exact-time".into()),
            ),
            "2026-10-07",
        )];
        append_and_replay(
            &store,
            &meet,
            &canonical_event,
            &performance,
            &format!("cen11:{date}:{raw}"),
        )?;
    }
    Ok(store)
}

fn append_and_replay(
    store: &Store,
    meet: &CanonicalMeet,
    event: &CanonicalEvent,
    performance: &CanonicalPerformance,
    operation: &str,
) -> TestResult {
    let digest = serde_json::to_string(performance)?;
    let mut batch = store.write_batch();
    batch.append_many(Table::Meets, std::slice::from_ref(meet))?;
    batch.append_many(Table::Events, std::slice::from_ref(event))?;
    batch.append_many(Table::Performances, std::slice::from_ref(performance))?;
    let first = batch.commit_once(operation, performance.id.as_str())?;
    check!(eq; first.appended(), 3);
    let before = store.stats()?;
    let mut replay = store.write_batch();
    replay.append_many(Table::Meets, std::slice::from_ref(meet))?;
    replay.append_many(Table::Events, std::slice::from_ref(event))?;
    replay.append_many(Table::Performances, std::slice::from_ref(performance))?;
    let repeated = replay.commit_once(operation, first.receipt().digest.as_str())?;
    check!(eq; repeated.appended(), 0);
    let after = store.stats()?;
    check!(eq; (after.tables, after.appended, after.observations), (before.tables, before.appended, before.observations));
    let recovered: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
    let recovered = recovered
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()?;
    check!(recovered.contains(&digest));
    Ok(())
}
