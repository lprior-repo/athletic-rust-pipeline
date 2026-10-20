use super::{codec, fixtures, journal, TestResult};
use serde_json::{json, Value};

#[derive(Clone, Copy)]
pub(super) enum Scenario {
    Omitted,
    Legacy,
    Managed,
    SelectedMissingExport,
    SelectedLegacyExport,
    SelectedMissingGeneration,
    LegacySelectedExport,
    ManagedSelectedExport,
    OmittedPublicationSelectedExport,
    ForeignStore,
    UnknownProfile,
    WrongJurisdictions,
    WrongSchoolYear,
    WrongAddressPin,
    SelectedExact,
}

#[tokio::test]
async fn omitted_national_contracts_are_refused_before_binding_or_stage_work() -> TestResult {
    super::exercise(super::Scenario::Admission(Scenario::Omitted)).await
}

#[tokio::test]
async fn legacy_national_contract_is_refused_before_binding_or_stage_work() -> TestResult {
    super::exercise(super::Scenario::Admission(Scenario::Legacy)).await
}

#[tokio::test]
async fn managed_national_contract_is_refused_before_binding_or_stage_work() -> TestResult {
    super::exercise(super::Scenario::Admission(Scenario::Managed)).await
}

#[tokio::test]
async fn selected_national_contract_requires_an_explicit_selected_export() -> TestResult {
    for scenario in [
        Scenario::SelectedMissingExport,
        Scenario::SelectedLegacyExport,
    ] {
        super::exercise(super::Scenario::Admission(scenario)).await?;
    }
    Ok(())
}

#[tokio::test]
async fn historical_selected_request_without_generation_cannot_execute_or_bind() -> TestResult {
    super::exercise(super::Scenario::Admission(
        Scenario::SelectedMissingGeneration,
    ))
    .await
}

#[tokio::test]
async fn selected_export_cannot_authorize_legacy_managed_or_omitted_national_contracts(
) -> TestResult {
    for scenario in [
        Scenario::LegacySelectedExport,
        Scenario::ManagedSelectedExport,
        Scenario::OmittedPublicationSelectedExport,
    ] {
        super::exercise(super::Scenario::Admission(scenario)).await?;
    }
    Ok(())
}
#[tokio::test]
async fn exact_preflight_refuses_foreign_or_unknown_selection_before_binding() -> TestResult {
    for scenario in [Scenario::ForeignStore, Scenario::UnknownProfile] {
        super::exercise(super::Scenario::Admission(scenario)).await?;
    }
    Ok(())
}

#[tokio::test]
async fn exact_preflight_refuses_mismatched_options_before_binding() -> TestResult {
    for scenario in [Scenario::WrongJurisdictions, Scenario::WrongSchoolYear] {
        super::exercise(super::Scenario::Admission(scenario)).await?;
    }
    Ok(())
}

#[tokio::test]
async fn exact_address_pin_refuses_before_binding() -> TestResult {
    super::exercise(super::Scenario::Admission(Scenario::WrongAddressPin)).await
}

#[tokio::test]
async fn exact_selected_request_proceeds_through_real_workbook() -> TestResult {
    super::exercise(super::Scenario::Admission(Scenario::SelectedExact)).await
}

pub(super) async fn run(scenario: Scenario, client: &reqwest::Client, url: &str) -> TestResult {
    if matches!(
        scenario,
        Scenario::ForeignStore
            | Scenario::UnknownProfile
            | Scenario::WrongJurisdictions
            | Scenario::WrongSchoolYear
            | Scenario::WrongAddressPin
            | Scenario::SelectedExact
    ) {
        return run_exact(scenario, client, url).await;
    }
    let mut journal = journal::Journal::new(&fixtures::request()?)?;
    let input = input(scenario)?;
    journal.entries = vec![codec::Frame::new(
        0x0400,
        codec::blob(14, &codec::value(&serde_json::to_vec(&input)?)?)?,
    )];
    let response = client
        .post(url)
        .header("content-type", "application/vnd.restate.invocation.v6")
        .body(journal.request_body()?)
        .send()
        .await?;
    check!(eq; response.status(), reqwest::StatusCode::OK);
    let frames = codec::frames(&response.bytes().await?)?;
    let output = frames
        .iter()
        .find(|frame| frame.kind == 0x0401)
        .ok_or("invalid national request did not return terminal output")?;
    let failure = codec::Frame::new(0, output.bytes(15)?.to_vec());
    check!(eq; failure.number(1)?, 400);
    check!(frames.iter().any(|frame| frame.kind == 0x0003));
    check!(
        !frames
            .iter()
            .any(|frame| (0x0402..0x8000).contains(&frame.kind)),
        "invalid admission emitted state, preflight, redrive, or stage commands: {frames:?}"
    );
    Ok(())
}

fn input(scenario: Scenario) -> TestResult<Value> {
    let mut input = serde_json::to_value(fixtures::request()?)?;
    let object = input
        .as_object_mut()
        .ok_or("national fixture is not an object")?;
    object.remove("publication_contract");
    object.remove("export_job");
    object.insert(
        "school_address".to_owned(),
        json!({
            "generation": "admission-must-not-preflight", "acquisitions": []
        }),
    );
    let publication = match scenario {
        Scenario::Omitted | Scenario::OmittedPublicationSelectedExport => None,
        Scenario::Legacy | Scenario::LegacySelectedExport => Some("legacy_separate_v1"),
        Scenario::Managed | Scenario::ManagedSelectedExport => Some("managed_bundle_v2"),
        Scenario::SelectedMissingExport
        | Scenario::SelectedLegacyExport
        | Scenario::ForeignStore
        | Scenario::UnknownProfile
        | Scenario::WrongJurisdictions
        | Scenario::WrongSchoolYear
        | Scenario::WrongAddressPin
        | Scenario::SelectedExact => Some("selected_publication_v2"),
        Scenario::SelectedMissingGeneration => Some("selected_publication_v1"),
    };
    if let Some(contract) = publication {
        object.insert("publication_contract".to_owned(), json!(contract));
    }
    match scenario {
        Scenario::LegacySelectedExport
        | Scenario::ManagedSelectedExport
        | Scenario::SelectedMissingGeneration
        | Scenario::OmittedPublicationSelectedExport => {
            let controlled = crate::restate_services::jobs::selected_fixture::install()
                .map_err(|error| error.to_string())?;
            let selected = fixtures::selected_request(
                &controlled,
                &census_domain::UsJurisdiction::CENSUS_SCOPE,
            )?;
            object.insert(
                "export_job".to_owned(),
                serde_json::to_value(selected.export_job)?,
            );
        }
        Scenario::Legacy | Scenario::Managed | Scenario::SelectedLegacyExport => {
            object.insert("export_job".to_owned(), json!({"contract": "legacy_v2"}));
        }
        Scenario::Omitted | Scenario::SelectedMissingExport => {}
        Scenario::ForeignStore
        | Scenario::UnknownProfile
        | Scenario::WrongJurisdictions
        | Scenario::WrongSchoolYear
        | Scenario::WrongAddressPin
        | Scenario::SelectedExact => {
            return Err("exact admission uses controlled request".into());
        }
    }
    Ok(input)
}
async fn run_exact(scenario: Scenario, client: &reqwest::Client, url: &str) -> TestResult {
    let controlled = crate::restate_services::jobs::selected_fixture::install()
        .map_err(|error| error.to_string())?;
    let request = exact_request(scenario, &controlled)?;
    let mut journal = journal::Journal::new(&request)?;
    journal.selected_workbook = Some(controlled);
    if matches!(scenario, Scenario::SelectedExact) {
        journal.drive(client, url).await?;
        check!(journal.output.is_some());
        check!(journal.terminal_failure.is_none());
        check!(has_call(&journal, "Census", "bind_run"));
        check!(has_call(&journal, "Workbook", "run"));
        check!(journal.selected_workbook_path.is_some());
        return Ok(());
    }
    let error = journal
        .drive(client, url)
        .await
        .err()
        .ok_or("exact preflight accepted")?;
    check!(journal.output.is_none());
    check!(journal.terminal_failure.is_some());
    check_refusal(&journal)?;
    let _ = error;
    Ok(())
}

fn has_call(journal: &journal::Journal, service: &str, handler: &str) -> bool {
    journal.calls.iter().any(|call| {
        call.text(1).is_ok_and(|name| name == service)
            && call.text(2).is_ok_and(|name| name == handler)
    })
}

fn check_refusal(journal: &journal::Journal) -> TestResult {
    check!(!journal
        .entries
        .iter()
        .any(|entry| matches!(entry.kind, 0x0402 | 0x0403)));
    check!(has_call(journal, "Census", "workbook_preflight"));
    check!(!has_call(journal, "Census", "bind_run"));
    check!(!has_call(journal, "JurisdictionCensus", "run"));
    check!(!has_call(journal, "Workbook", "run"));
    check!(journal.selected_workbook_path.is_none());
    Ok(())
}
fn exact_request(
    scenario: Scenario,
    controlled: &crate::restate_services::jobs::selected_fixture::Controlled,
) -> TestResult<crate::restate_services::wire::NationalRequest> {
    let mut request =
        fixtures::selected_request(controlled, &[census_domain::UsJurisdiction::Wisconsin])?;
    match scenario {
        Scenario::ForeignStore => {
            let foreign = census_domain::model::StoreIdentity::parse(&"0".repeat(64))?;
            let selection =
                census_domain::model::ProfileSelectionV1::new(foreign, controlled.profile().id()?);
            request.export_job =
                census_report::export::ExportJobRequest::SelectedPublicationV1 { selection };
        }
        Scenario::UnknownProfile => {
            let unknown = census_domain::model::ProfileId::parse(&"f".repeat(64))?;
            let selection = census_domain::model::ProfileSelectionV1::new(
                controlled.store().store_identity()?,
                unknown,
            );
            request.export_job =
                census_report::export::ExportJobRequest::SelectedPublicationV1 { selection };
        }
        Scenario::WrongJurisdictions => {
            request.jurisdictions = vec![census_domain::UsJurisdiction::Colorado];
        }
        Scenario::WrongSchoolYear => {
            request.season =
                census_domain::model::SchoolYear::new(2025).ok_or("fixture school year")?;
        }
        Scenario::WrongAddressPin => {
            request.school_address = Some(serde_json::from_value(json!({
                "generation": "original-directory",
                "acquisitions": [],
                "expected_digest": "wrong-address-pin"
            }))?);
        }
        Scenario::SelectedExact => {}
        _ => return Err("exact admission cannot build shape request".into()),
    }
    Ok(request)
}
