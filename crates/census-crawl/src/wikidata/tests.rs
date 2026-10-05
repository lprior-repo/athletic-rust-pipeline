use super::*;

const FIXTURE: &str = include_str!("../../tests/fixtures/wikidata/schools.json");

#[test]
fn parse_sparql_accepts_valid_wdqs_json_and_maps_rows() -> anyhow::Result<()> {
    let outcome = parse_sparql(FIXTURE)?;
    let entries = outcome.entries();
    check!(eq; entries.len(), 4, "three valid rows plus one with no state");
    let first = entries
        .first()
        .ok_or_else(|| anyhow::anyhow!("the fixture carries a first entry"))?;
    check!(
        eq;
        first.name().map(|name| name.as_str()),
        Some("Lincoln High School")
    );
    check!(
        eq;
        first.website().map(|site| site.as_str()),
        Some("https://www.lincolnhs.edu")
    );
    Ok(())
}

#[test]
fn parse_sparql_skips_rows_missing_required_fields() -> anyhow::Result<()> {
    let outcome = parse_sparql(FIXTURE)?;
    let skipped = outcome.skipped();
    check!(
        skipped.iter().any(|issue| issue.field == "website"),
        "the row with no website is skipped"
    );
    check!(
        skipped
            .iter()
            .any(|issue| issue.field == "website" && issue.detail.contains("not a url")),
        "the row with an invalid website is skipped"
    );
    Ok(())
}

#[test]
fn parse_sparql_deduplicates_by_qid() -> anyhow::Result<()> {
    let outcome = parse_sparql(FIXTURE)?;
    let notes = outcome.notes();
    check!(
        notes
            .iter()
            .any(|issue| issue.field == "item" && issue.detail.contains("duplicate")),
        "duplicate QID is noted"
    );
    let entries = outcome.entries();
    let roosevelt = entries
        .iter()
        .filter(|entry| {
            entry
                .name()
                .is_some_and(|name| name.as_str().contains("Roosevelt"))
        })
        .count();
    check!(eq; roosevelt, 1, "only one Roosevelt entry survives");
    Ok(())
}

#[test]
fn parse_sparql_rejects_missing_required_vars() -> anyhow::Result<()> {
    let json = r#"{"head":{"vars":["item","itemLabel"]},"results":{"bindings":[]}}"#;
    let Err(error) = parse_sparql(json) else {
        return Err(anyhow::anyhow!("a missing website column must be refused"));
    };
    match error {
        crate::CrawlError::Schema { detail, .. } => {
            check!(detail.contains("website"));
            Ok(())
        }
        other => Err(anyhow::anyhow!("expected a schema error, got {other:?}")),
    }
}

#[test]
fn sparql_query_contains_expected_structure() -> anyhow::Result<()> {
    let query = sparql_query();
    check!(query.contains("SELECT"));
    check!(query.contains("?item"));
    check!(query.contains("wdt:P856"));
    check!(query.contains("LIMIT"));
    Ok(())
}
