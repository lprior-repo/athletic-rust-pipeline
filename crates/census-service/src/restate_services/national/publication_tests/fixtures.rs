use super::codec::Frame;
use super::journal::Journal;
use super::TestResult;
use crate::restate_services::wire::NationalRequest;
use serde_json::{json, Value};

pub(super) const MANAGED_WORKBOOK: &str = "out/publication/generations/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/workbook.xlsx";

pub fn request() -> TestResult<NationalRequest> {
    Ok(serde_json::from_value(json!({
        "season": 2026, "revision": 1,
        "history": {"first_calendar_year": 2023, "last_calendar_year": 2026, "as_of": "2026-10-12"},
        "pass_budget": 0
    }))?)
}

pub fn answer(call: &Frame, journal: &Journal) -> TestResult<Value> {
    let request: Value = serde_json::from_slice(call.bytes(3)?)?;
    let service = call.text(1)?;
    let handler = call.text(2)?;
    if service == "TeamsSource" && handler == "contacts" {
        if let Some(store) = &journal.source_store {
            let input = serde_json::from_value(request)?;
            let outcome =
                crate::restate_services::jurisdiction::team_source::tests::national_resume::replay(
                    store,
                    &call.text(5)?,
                    &input,
                )?;
            return Ok(serde_json::to_value(outcome)?);
        }
    }
    Ok(match (service.as_str(), handler.as_str()) {
        ("Census", "school_address_preflight") => json!(journal.address_digest),
        ("Census", "bind_run") => json!({
            "store_identity": "fixture-store", "season": 2026, "revision": 1,
            "cohort": 2027, "jurisdictions": required(&request, "jurisdictions")?
        }),
        ("JurisdictionCensus", "run") => json!({
            "identity": call.text(5)?, "jurisdiction": required(&request, "jurisdiction")?, "plan": {},
            "stages_run": ["teams", "rosters", "meets", "results"], "teams": 1,
            "rosters": { "state": required(&request, "jurisdiction")?, "teams": 1,
                "rosters_total": 1, "rosters_committed": if journal.unfinished_rosters {0} else {1},
                "rosters_remaining": usize::from(journal.unfinished_rosters), "rosters_skipped": 0,
                "blocked": false, "blocked_skipped": 0,
                "athletes": 1, "class_of_2027": 1, "class_of_2027_boys": 1,
                "class_of_2027_girls": 0, "errors": [] },
            "consolidated": [], "history": {"meets": {}, "results": {}},
            "history_window": required(&request, "history")?, "completed_at": "2026-10-12"
        }),
        ("TeamsSource", "contacts") => json!({
            "status": "completed", "progress": [],
            "outcome": {"records": 1, "at": "2026-10-12", "errors": [], "notes": [],
                "disposition": if journal.partial_contacts {"partial"} else {"complete"}, "unfinished": []}
        }),
        ("Consolidate", "run") => json!({"tables": []}),
        ("Report", "run") => json!({
            "scope": "all_sources", "generated_on": "2026-10-12", "totals": {},
            "json_path": "out/census-all_sources.json", "csv_path": "out/census-all_sources.csv"
        }),
        ("Bests", "run") => json!({
            "cohort": "2027", "rows": 49, "jsonl": "out/bests-2027.jsonl", "csv": "out/bests-2027.csv"
        }),
        ("Workbook", "run") => {
            let path = if journal.original.publication_contract
                != crate::restate_services::PublicationContract::LegacySeparateV1
            {
                MANAGED_WORKBOOK
            } else {
                "out/publication/current/workbook.xlsx"
            };
            json!({"path": path, "grad_year": 2027})
        }
        ("Census", "publication") => json!({
            "phase": "sealed", "workbook": required(&request, "workbook")?, "recorded": null,
            "sealed": {"digest": "fixture-publication-digest", "sealed_on": "2026-10-12"},
            "open": [], "refusal": null, "wrote": "out/seal.json",
            "counts": {"jurisdiction_buckets": 49, "schools": 49, "meets": 49, "athletes": 49,
                "class_of_2027": 49, "cohort_performances": 49, "coaches": 49},
            "retained": {"gaps": [], "conflicts": 0, "access_conditions": 0, "blocked_hosts": 0,
                "throttled_hosts": 0, "silent_sources": [], "source_failures": 0,
                "observations": 49, "calculations": 49}
        }),
        _ => return Err(format!("unexpected national child {service}/{handler}").into()),
    })
}

fn required<'a>(request: &'a Value, name: &str) -> TestResult<&'a Value> {
    request
        .get(name)
        .ok_or_else(|| format!("fixture child request has no {name}").into())
}
