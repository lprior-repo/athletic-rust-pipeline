use super::*;
use crate::participation_evidence::{RetainedBodyEncodingV1, RetainedHttpProvenanceV1};
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const URL: &str = "https://mi.milesplit.com/athletes/7223777-zacchaeus-brocks";
const BODY: &[u8] = include_bytes!("../tests/fixtures/profile.html");

fn genuine() -> TestResult<RetainedHtmlCaptureV1> {
    let acquired = DateTime::parse_from_rfc3339("2026-10-16T00:50:27Z")?.with_timezone(&Utc);
    Ok(RetainedHtmlCaptureV1::new(
        URL,
        URL,
        200,
        acquired,
        BODY,
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: vec![
                ("User-Agent".into(), "OpenAI-research/0.1".into()),
                ("Accept".into(), "text/html".into()),
            ],
            response_header_bytes: include_bytes!("../tests/fixtures/profile.headers").to_vec(),
            content_type: "text/html; charset=UTF-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: DateTime::parse_from_rfc3339("2026-10-16T00:50:21Z")?.with_timezone(&Utc),
        },
    )?)
}

fn controlled(url: &str, html: &str) -> TestResult<RetainedHtmlCaptureV1> {
    let mut value = serde_json::to_value(genuine()?)?;
    use base64::{engine::general_purpose::STANDARD, Engine};
    value["requested_url"] = url.into();
    value["final_url"] = url.into();
    value["body_bytes"] = html.len().into();
    value["body_sha256"] = hash(html.as_bytes()).into();
    value["body_base64"] = STANDARD.encode(html).into();
    Ok(serde_json::from_value(value)?)
}

#[test]
fn genuine_table_retains_all_public_filter_rows_and_exact_capture_locators() -> TestResult {
    let capture = genuine()?;
    check!(eq; capture.body_sha256(), "b11d89fd95b7e08e5a90d1cbc1e51174cdb4a5f1bdf3833b980f4256ae1b266c");
    let table = read(&capture, UsJurisdiction::Michigan, "7223777")?;
    check!(eq; table.rows().len(), 46);
    check!(eq; table.athlete().id.as_str(), "7223777");
    check!(eq; table.capture().body_sha256(), capture.body_sha256());
    let mut counts = BTreeMap::new();
    let html = std::str::from_utf8(BODY)?;
    for row in table.rows() {
        *counts
            .entry((row.source_mode(), row.source_level()))
            .or_insert(0) += 1;
        let locator = row.locator();
        let end = locator
            .byte_offset()
            .checked_add(locator.byte_length())
            .ok_or("span")?;
        let source = html.get(locator.byte_offset()..end).ok_or("source slice")?;
        check!(source.starts_with("<tr "));
        check!(source.ends_with("</tr>"));
        check!(source.contains(row.event()));
        check!(source.contains(row.published_mark()));
        check!(source.contains(row.published_date()));
    }
    check!(eq; counts, BTreeMap::from([
        (("all", "all"), 5), (("indoor", "all"), 5), (("indoor", "hs"), 5),
        (("outdoor", "all"), 12), (("outdoor", "hs"), 6),
        (("outdoor", "ms"), 5), (("outdoor", "open"), 1), (("outdoor", "youth"), 7)
    ]));
    let indoor = table
        .rows()
        .iter()
        .filter(|row| {
            row.event() == "200m"
                && row.published_mark() == "22.23"
                && row.published_date() == "Feb 15, 2025"
        })
        .map(|row| (row.source_mode(), row.source_level()))
        .collect::<Vec<_>>();
    check!(eq; indoor, vec![("all", "all"), ("indoor", "all"), ("indoor", "hs")]);
    let replayed: RetainedHtmlCaptureV1 = serde_json::from_slice(&serde_json::to_vec(&capture)?)?;
    let repeated = read(&replayed, UsJurisdiction::Michigan, "7223777")?;
    check!(eq; repeated.reference(), table.reference());
    check!(eq; repeated.rows(), table.rows());
    Ok(())
}

#[test]
fn controlled_mutations_refuse_foreign_owner_capture_corruption_and_ambiguous_cells() -> TestResult
{
    let capture = genuine()?;
    check!(read(&capture, UsJurisdiction::Iowa, "7223777").is_err());
    check!(read(&capture, UsJurisdiction::Michigan, "7223778").is_err());
    let html = std::str::from_utf8(BODY)?;
    for changed in [
        html.replacen("id=\"prTable\"", "id=\"differentTable\"", 1),
        html.replacen("data-pr-level=\"all\"", "data-pr-unknown=\"all\"", 1),
        html.replacen("pr-td-date", "wrong-date-cell", 1),
        html.replacen("pr-td-mark", "wrong-mark-cell", 1),
        html.replacen("pr-td-event", "wrong-event-cell", 1),
        html.replacen("data-pr-mode=\"all\"", "data-pr-mode=\"\"", 1),
        html.replacen("data-pr-mode=\"all\"", "data-pr-unknown=\"all\"", 1),
        html.replacen("class=\"pr-td-date\"", "class=\"pr-td-mark\"", 1),
        html.replacen(
            "https://mi.milesplit.com/athletes/7223777-zacchaeus-brocks\" />",
            "https://mi.milesplit.com/athletes/7223778-other\" />",
            1,
        ),
    ] {
        let changed = controlled(URL, &changed)?;
        check!(read(&changed, UsJurisdiction::Michigan, "7223777").is_err());
    }
    let mut corrupt = serde_json::to_value(&capture)?;
    corrupt["body_sha256"] = "0".repeat(64).into();
    let corrupt = serde_json::from_value(corrupt)?;
    check!(read(&corrupt, UsJurisdiction::Michigan, "7223777").is_err());
    Ok(())
}

#[test]
fn controlled_stats_request_keeps_root_canonical_and_rejects_table_substitutions() -> TestResult {
    let html = std::str::from_utf8(BODY)?;
    let stats = controlled(&format!("{URL}/stats"), html)?;
    let table = read(&stats, UsJurisdiction::Michigan, "7223777")?;
    check!(eq; table.rows().len(), 46);
    for fragment in [
        "<table id=\"prTable\" class=\"pr-table\"></table>",
        "<table id=\"other\" class=\"pr-table\"></table>",
    ] {
        let changed = controlled(URL, &format!("{html}{fragment}"))?;
        check!(read(&changed, UsJurisdiction::Michigan, "7223777").is_err());
    }
    Ok(())
}

#[test]
fn controlled_row_limit_refuses_one_more_than_256_without_truncation() -> TestResult {
    let row = "<tr data-pr-mode=\"outdoor\" data-pr-level=\"hs\"><td class=\"pr-td-event\">100m</td><td class=\"pr-td-mark\">12.34</td><td class=\"pr-td-date\">May 2, 2025</td></tr>";
    for (count, allowed) in [(256, true), (257, false)] {
        let html = format!("<link rel=\"canonical\" href=\"{URL}\"><table id=\"prTable\" class=\"pr-table\">{}</table>", row.repeat(count));
        let capture = controlled(URL, &html)?;
        let result = read(&capture, UsJurisdiction::Michigan, "7223777");
        check!(eq; result.is_ok(), allowed);
        if let Ok(table) = result {
            check!(eq; table.rows().len(), count);
        }
    }
    Ok(())
}
