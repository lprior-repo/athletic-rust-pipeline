use super::TestResult;
use calamine::{Data, Reader, Xlsx};
use census_domain::model::{CanonicalMeet, CanonicalPerformance, SourceNamespace};
use census_domain::UsJurisdiction;
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn verify(
    book: &mut Xlsx<std::io::BufReader<std::fs::File>>,
    original_meets: &Value,
    performances: &[CanonicalPerformance],
) -> TestResult {
    let directory = book.worksheet_range("Meets")?;
    check!(eq; directory.height(), 1);
    check!(directory
        .rows()
        .next()
        .ok_or("meet headers")?
        .iter()
        .any(|cell| cell.to_string() == "Meet ID"));
    check!(eq; original_meets.as_array().ok_or("retained meet array")?.len(), 2);
    let meets: Vec<CanonicalMeet> = serde_json::from_value(original_meets.clone())?;
    check!(eq; meets.iter().map(|meet| &meet.id).collect::<BTreeSet<_>>().len(), 2);
    check!(eq; performances.len(), 2);
    let range = book.worksheet_range("Performances_001")?;
    check!(eq; range.height(), 3);
    check!(range.width() <= 128);
    let headers = range.rows().next().ok_or("performance headers")?;
    check!(eq; headers.iter().map(ToString::to_string).collect::<BTreeSet<_>>().len(), headers.len());
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
        let mut found = None;
        for row in range.rows().skip(1) {
            if cell(headers, row, "Canonical Result ID")? == performance.id.as_str() {
                check!(found.replace(row).is_none());
            }
        }
        performance_row(
            headers,
            found.ok_or("published historical result")?,
            performance,
            meet,
        )?;
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

fn performance_row(
    headers: &[Data],
    row: &[Data],
    performance: &CanonicalPerformance,
    meet: &CanonicalMeet,
) -> TestResult {
    let source = performance
        .evidence
        .first()
        .ok_or("original result source")?;
    let source_url = source
        .source
        .url
        .as_deref()
        .ok_or("owned result capture URL")?;
    for (field, expected) in [
        ("Meet ID", meet.id.as_str()),
        ("Meet", meet.name.as_str()),
        ("Date", meet.date.as_str()),
        ("State", "AL"),
        ("School", "Synthetic selected school"),
        ("Retained Candidate ID", performance.athlete.as_str()),
        ("Source", source.source.id.as_str()),
        ("Source URL", source_url),
        ("Source ResultID", performance.source_key.as_str()),
    ] {
        check!(eq; cell(headers, row, field)?, expected, "{field}");
    }
    Ok(())
}

fn cell(headers: &[Data], row: &[Data], name: &str) -> TestResult<String> {
    let column = headers
        .iter()
        .position(|cell| cell.to_string() == name)
        .ok_or_else(|| format!("missing {name} column"))?;
    Ok(row.get(column).ok_or("historical venue cell")?.to_string())
}
