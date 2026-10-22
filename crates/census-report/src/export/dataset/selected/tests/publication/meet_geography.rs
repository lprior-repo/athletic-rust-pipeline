use super::TestResult;
use calamine::{Reader, Xlsx};
use census_domain::model::{CanonicalMeet, CanonicalPerformance, SourceNamespace};
use census_domain::UsJurisdiction;
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn verify(
    book: &mut Xlsx<std::io::BufReader<std::fs::File>>,
    original_meets: &Value,
    performances: &[CanonicalPerformance],
) -> TestResult {
    check!(eq; book.sheet_names(), vec!["Athletes".to_owned()]);
    check!(eq; original_meets.as_array().ok_or("retained meet array")?.len(), 2);
    let meets: Vec<CanonicalMeet> = serde_json::from_value(original_meets.clone())?;
    check!(eq; meets.iter().map(|meet| &meet.id).collect::<BTreeSet<_>>().len(), 2);
    check!(eq; performances.len(), 2);
    for (key, name, date, owner, url) in [
        (
            "milesplit_result:201782263",
            "Troy Invitational #2",
            "2026-03-27",
            "725218",
            "https://al.milesplit.com/meets/725218/results/1266814/raw",
        ),
        (
            "milesplit_result:201782264",
            "Troy Invitational #3",
            "2026-04-03",
            "725219",
            "https://al.milesplit.com/meets/725219/results/1266815/raw",
        ),
    ] {
        let performance = performances
            .iter()
            .find(|row| row.source_key == key)
            .ok_or("retained result")?;
        let meet = meets
            .iter()
            .find(|meet| meet.id == performance.meet)
            .ok_or("retained venue")?;
        venue(meet, name, date, owner, url)?;
        source(performance, name, date, url)?;
        check!(eq; performance.date, meet.date);
    }
    Ok(())
}

fn venue(meet: &CanonicalMeet, name: &str, date: &str, owner: &str, url: &str) -> TestResult {
    check!(eq; meet.name, name);
    check!(eq; meet.date, date);
    check!(eq; meet.state, Some(UsJurisdiction::Alabama));
    check!(eq; meet.source_urls, vec![url.to_owned()]);
    check!(eq; meet.source_identities.len(), 1);
    let identity = meet
        .source_identities
        .first()
        .ok_or("retained meet owner")?;
    check!(eq; identity.namespace, SourceNamespace::MilesplitMeet);
    check!(eq; identity.id, owner);
    check!(eq; meet.evidence.len(), 1);
    check!(eq; meet.evidence.first().ok_or("retained meet evidence")?.source.url.as_deref(), Some(url));
    Ok(())
}

fn source(performance: &CanonicalPerformance, name: &str, date: &str, url: &str) -> TestResult {
    let evidence = performance
        .evidence
        .first()
        .ok_or("retained result evidence")?;
    let note: Value = serde_json::from_str(evidence.note.as_deref().ok_or("owned result note")?)?;
    for (field, expected) in [
        ("raw_metadata_url", url),
        ("meet_name", name),
        ("meet_date", date),
    ] {
        check!(eq; note.get(field).and_then(Value::as_str), Some(expected));
    }
    check!(eq; note.get("team_id").and_then(Value::as_u64), Some(38332));
    check!(eq; note.get("capture_url").and_then(Value::as_str), evidence.source.url.as_deref());
    Ok(())
}
