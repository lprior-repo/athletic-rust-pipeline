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
        "publication_contract": "selected_publication_v2",
        "export_job": {"contract": "selected_publication_v1",
            "selection": {"version": 1,
                "store_identity": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "profile_id": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}},
        "pass_budget": 0
    }))?)
}

pub(in crate::restate_services::national) fn selected_request(
    controlled: &crate::restate_services::jobs::selected_fixture::Controlled,
    jurisdictions: &[census_domain::UsJurisdiction],
) -> TestResult<NationalRequest> {
    let mut input = controlled.profile().input().clone();
    input.jurisdictions = jurisdictions.to_vec();
    let profile = census_domain::model::QueryProfileV1::new(input)?;
    controlled.store().register_profile(&profile)?;
    let mut request = request()?;
    request.jurisdictions = jurisdictions.to_vec();
    request.publication_contract =
        crate::restate_services::PublicationContract::SelectedPublicationV2;
    request.export_job = census_report::export::ExportJobRequest::SelectedPublicationV1 {
        selection: census_domain::model::ProfileSelectionV1::new(
            controlled.store().store_identity()?,
            profile.id()?,
        ),
    };
    request.generation = Some(crate::restate_services::ExportGeneration::default_generation());
    Ok(request)
}
pub(super) fn workbook_preflight(
    call: &Frame,
    journal: &Journal,
) -> TestResult<Result<Value, restate_sdk::prelude::HandlerError>> {
    let controlled = journal
        .selected_workbook
        .as_ref()
        .ok_or("selected National dispatch requires its controlled workbook fixture")?;
    let request: crate::restate_services::wire::WorkbookRequest =
        serde_json::from_slice(call.bytes(3)?)?;
    match crate::restate_services::publish::workbook_preflight::checked_options(
        controlled.store(),
        &request,
    ) {
        Ok(_) => Ok(Ok(Value::Null)),
        Err(error) => Ok(Err(error)),
    }
}

pub fn answer(call: &Frame, journal: &mut Journal) -> TestResult<Value> {
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
    if service == "Census" && handler == "bind_run" {
        return bind_current_run(request, journal);
    }
    if service == "Workbook" && handler == "run" {
        return selected_workbook(call, request, journal);
    }
    stage_answer(call, journal, &request, &service, &handler)
}

fn stage_answer(
    call: &Frame,
    journal: &Journal,
    request: &Value,
    service: &str,
    handler: &str,
) -> TestResult<Value> {
    Ok(match (service, handler) {
        ("Census", "school_address_preflight") => json!(journal.address_digest),
        ("JurisdictionCensus", "run") => json!({
            "identity": call.text(5)?, "jurisdiction": required(request, "jurisdiction")?, "plan": {},
            "stages_run": ["teams", "rosters", "meets", "results"], "teams": 1,
            "rosters": { "state": required(request, "jurisdiction")?, "teams": 1,
                "rosters_total": 1, "rosters_committed": if journal.unfinished_rosters {0} else {1},
                "rosters_remaining": usize::from(journal.unfinished_rosters), "rosters_skipped": 0,
                "blocked": false, "blocked_skipped": 0,
                "athletes": 1, "class_of_2027": 1, "class_of_2027_boys": 1,
                "class_of_2027_girls": 0, "errors": [] },
            "consolidated": [], "history": {"meets": {}, "results": {}},
            "history_window": required(request, "history")?, "completed_at": "2026-10-12"
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
            json!({"path": MANAGED_WORKBOOK, "grad_year": 2027})
        }
        ("Census", "publication") => json!({
            "phase": "sealed", "workbook": required(request, "workbook")?, "recorded": null,
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

fn bind_current_run(request: Value, journal: &Journal) -> TestResult<Value> {
    let controlled = journal
        .selected_workbook
        .as_ref()
        .ok_or("selected run binding requires its controlled source fixture")?;
    let request = serde_json::from_value(request)?;
    let reply = crate::restate_services::census::binding::bind(controlled.store(), request)?;
    Ok(serde_json::to_value(reply)?)
}

fn selected_workbook(call: &Frame, request: Value, journal: &mut Journal) -> TestResult<Value> {
    let controlled = journal
        .selected_workbook
        .as_ref()
        .ok_or("selected National dispatch requires its controlled workbook fixture")?;
    let workbook_request: crate::restate_services::wire::WorkbookRequest =
        serde_json::from_value(request)?;
    check!(eq;
        serde_json::to_value(&workbook_request.export_job)?,
        serde_json::to_value(&journal.original.export_job)?,
        "National dispatch must retain the controlled store and profile selection"
    );
    let grad_year = workbook_request
        .grad_year
        .and_then(census_domain::model::GradYear::new)
        .ok_or("National selected workbook omitted its graduation cohort")?;
    let school_year = workbook_request
        .school_year
        .and_then(census_domain::model::SchoolYear::new)
        .ok_or("National selected workbook omitted its school year")?;
    let options = census_report::workbook::Options {
        address_corpus: None,
        grad_year: Some(grad_year.get()),
        out: workbook_request.out.map(std::path::PathBuf::from),
        limit: workbook_request.limit,
        scope: crate::restate_services::resolve_scope(workbook_request.scope.as_deref())
            .map_err(|error| error.to_string())?,
        jurisdictions: crate::restate_services::normalize_jurisdictions(
            &workbook_request.jurisdictions,
        )
        .map_err(|error| error.to_string())?,
        school_year,
    };
    let job = format!("workbook:{}", call.text(5)?);
    let reply = crate::restate_services::jobs::workbook_job::build(
        controlled.store(),
        &options,
        &job,
        &workbook_request.export_job,
    )?;
    journal.selected_workbook_path = Some(reply.path.clone());
    Ok(serde_json::to_value(reply)?)
}

fn required<'a>(request: &'a Value, name: &str) -> TestResult<&'a Value> {
    request
        .get(name)
        .ok_or_else(|| format!("fixture child request has no {name}").into())
}
