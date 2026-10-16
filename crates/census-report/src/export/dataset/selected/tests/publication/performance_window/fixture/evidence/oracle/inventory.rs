use super::super::super::results::{ResultCase, CASES};
use super::{CanonicalEvent, CanonicalMeet, CanonicalPerformance, TestResult};
use census_domain::model::{
    EventKind, EventSpecification, Gender, SourceIdentity, SourceNamespace, Sport,
};
use census_domain::UsJurisdiction;

pub(super) fn check(
    performances: &[CanonicalPerformance],
    events: &[CanonicalEvent],
    meets: &[CanonicalMeet],
) -> TestResult {
    check!(eq; performances.len(), 2, "fixture canonical performance count");
    check!(eq; events.len(), 2, "fixture meet-scoped canonical event count");
    check!(eq; meets.len(), 2, "fixture canonical meet count");
    let [march, april] = &CASES;
    let first = related(performances, events, meets, march)?;
    let second = related(performances, events, meets, april)?;
    check!(ne; first.id, second.id, "fixture distinct performance identities");
    check!(ne; first.event, second.event, "fixture distinct meet-scoped event identities");
    check!(ne; first.meet, second.meet, "fixture distinct meet identities");
    check!(eq; first.athlete, second.athlete, "fixture common athlete identity");
    check!(eq; first.team, second.team, "fixture common team identity");
    Ok(())
}

fn related<'a>(
    performances: &'a [CanonicalPerformance],
    events: &[CanonicalEvent],
    meets: &[CanonicalMeet],
    case: &ResultCase,
) -> TestResult<&'a CanonicalPerformance> {
    let source_key = format!("milesplit_result:{}", case.id);
    let performance = performances
        .iter()
        .find(|row| row.source_key == source_key)
        .ok_or_else(|| format!("fixture performance missing: {source_key}"))?;
    let event = events
        .iter()
        .find(|row| row.id == performance.event)
        .ok_or_else(|| format!("fixture event missing: {source_key}"))?;
    let meet = meets
        .iter()
        .find(|row| row.id == performance.meet)
        .ok_or_else(|| format!("fixture meet missing: {source_key}"))?;
    check!(eq; performance.date, case.date, "fixture performance date: {source_key}");
    check!(eq; event.meet, meet.id, "fixture event-to-performance meet: {source_key}");
    check!(eq; meet.name, case.name, "fixture meet name: {source_key}");
    check!(eq; meet.date, case.date, "fixture meet date: {source_key}");
    check!(eq; meet.source_identities, vec![SourceIdentity::new(SourceNamespace::MilesplitMeet, case.meet)], "fixture provider meet owner: {source_key}");
    check!(eq; meet.source_urls, vec![case.url()], "fixture result document URL: {source_key}");
    check!(eq; meet.state, Some(UsJurisdiction::Michigan), "fixture meet jurisdiction: {source_key}");
    check!(eq; meet.sports, vec![Sport::OutdoorTrack], "fixture meet sport: {source_key}");
    check!(eq; event.kind, EventKind::LongJump, "fixture event kind: {source_key}");
    check!(eq; event.gender, Gender::Boys, "fixture event category: {source_key}");
    check!(eq; event.division.as_deref(), Some("High School"), "fixture event division: {source_key}");
    check!(eq; event.round.as_deref(), Some("Finals"), "fixture event round: {source_key}");
    check!(eq; event.specification, EventSpecification::default(), "fixture published event specification: {source_key}");
    Ok(performance)
}
