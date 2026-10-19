use super::*;

pub(super) async fn run_scenario(scenario: Scenario, url: &str) -> TestResult {
    let client = reqwest::Client::builder()
        .http2_prior_knowledge()
        .timeout(Duration::from_secs(5))
        .build()?;
    match scenario {
        Scenario::Address(mode) => return binding::run(mode, &client, url).await,
        Scenario::Admission(mode) => return admission::run(mode, &client, url).await,
        Scenario::SealBudgetExhausted => return seal_budget::run(&client, url).await,
        _ => {}
    }
    let controlled = crate::restate_services::jobs::selected_fixture::install()
        .map_err(|error| error.to_string())?;
    let mut request =
        fixtures::selected_request(&controlled, &census_domain::UsJurisdiction::CENSUS_SCOPE)?;
    if matches!(scenario, Scenario::RefusedSeal) {
        request.pass_budget = 1;
    }
    let mut journal = journal::Journal::new(&request)?;
    journal.selected_workbook = Some(controlled);
    match scenario {
        Scenario::FailedStage(stage) => journal.hold_stage = Some(stage.to_owned()),
        Scenario::RefusedSeal => journal.hold_stage = Some("Census".to_owned()),
        Scenario::PartialContacts | Scenario::ExhaustedResume => journal.partial_contacts = true,
        Scenario::UnfinishedRosters | Scenario::CompletedResume => {
            journal.unfinished_rosters = true
        }
        Scenario::InvalidResume(suffix) => {
            journal.identity.push_str(&format!(":resume:{suffix}"));
            let error = journal
                .drive(&client, url)
                .await
                .err()
                .ok_or("invalid resume key accepted")?;
            check!(error.to_string().contains("request identity"), "{error}");
            check!(eq; journal.calls, Vec::<codec::Frame>::new());
            return Ok(());
        }
        Scenario::Complete => {}
        Scenario::Address(_) | Scenario::Admission(_) | Scenario::SealBudgetExhausted => {
            return Err("scenario dispatch failed".into());
        }
    }
    journal.drive(&client, url).await?;
    if matches!(
        scenario,
        Scenario::CompletedResume | Scenario::ExhaustedResume
    ) {
        journal = resume_sources(scenario, journal, &client, url).await?;
    }
    match scenario {
        Scenario::FailedStage(stage) => resume_stage(&mut journal, &client, url, stage).await?,
        Scenario::RefusedSeal => refresh_seal(&mut journal, &client, url).await?,
        Scenario::ExhaustedResume => return assert_exhaustion(&journal),
        Scenario::PartialContacts | Scenario::UnfinishedRosters => {
            check!(eq; publication_services(&journal)?, Vec::<String>::new());
            check!(eq; journal.output.as_ref().and_then(|row| row.get("jurisdictions")).and_then(serde_json::Value::as_array).map(Vec::len), Some(49));
            return Ok(());
        }
        Scenario::Complete | Scenario::CompletedResume => {}
        Scenario::InvalidResume(_)
        | Scenario::Address(_)
        | Scenario::Admission(_)
        | Scenario::SealBudgetExhausted => {
            return Err("nonpublication scenario reached completion".into());
        }
    }
    assert_published(&journal)
}

async fn resume_sources(
    scenario: Scenario,
    mut first: journal::Journal,
    client: &reqwest::Client,
    url: &str,
) -> TestResult<journal::Journal> {
    let retained = tempfile::tempdir()?;
    check!(eq; publication_services(&first)?, Vec::<String>::new());
    check!(first.output.is_some());
    let source_store = if matches!(scenario, Scenario::ExhaustedResume) {
        let store = census_store::Store::open(retained.path())?;
        exhaust_sources(&store, &first.calls)?;
        store.flush()?;
        drop(store);
        Some(Arc::new(census_store::Store::open(retained.path())?))
    } else {
        None
    };
    let mut resumed = journal::Journal::new(&first.original)?;
    resumed.identity.push_str(":resume:1");
    resumed.invocation = "invocation-national-resume-fixture".to_owned();
    resumed.source_store = source_store;
    resumed.selected_workbook = first.selected_workbook.take();
    resumed.drive(client, url).await?;
    check!(eq; source_calls(&resumed.calls)?, source_calls(&first.calls)?);
    let calls = resumed.calls.clone();
    let body = resumed.request_body()?;
    let output = resumed.output.clone();
    resumed.step(client, url).await?;
    check!(eq; resumed.calls, calls);
    check!(eq; resumed.request_body()?, body);
    check!(eq; resumed.output, output);
    resumed.source_store = None;
    Ok(resumed)
}

fn exhaust_sources(store: &census_store::Store, calls: &[codec::Frame]) -> TestResult {
    for call in calls {
        if call.text(1)? == "TeamsSource" && call.text(2)? == "contacts" {
            let request = serde_json::from_slice(call.bytes(3)?)?;
            crate::restate_services::jurisdiction::team_source::tests::national_resume::exhaust(
                store,
                &call.text(5)?,
                &request,
            )?;
        }
    }
    Ok(())
}

fn source_calls(calls: &[codec::Frame]) -> TestResult<Vec<(String, Vec<u8>)>> {
    calls
        .iter()
        .filter(|call| {
            call.text(1)
                .is_ok_and(|service| service == "JurisdictionCensus" || service == "TeamsSource")
        })
        .map(|call| Ok((call.text(5)?, call.bytes(3)?.to_vec())))
        .collect()
}

fn assert_exhaustion(journal: &journal::Journal) -> TestResult {
    check!(eq; publication_services(journal)?, Vec::<String>::new());
    let state = journal
        .entries
        .iter()
        .find(|entry| {
            entry.kind == 0x0403 && entry.text(1).is_ok_and(|key| key == "school_contacts")
        })
        .ok_or("resumed national workflow lost its contact budget outcomes")?;
    let bytes = codec::field(state.bytes(3)?, 1)?.ok_or("contact state value missing")?;
    let contacts: Vec<crate::restate_services::wire::ContactSummary> =
        serde_json::from_slice(bytes)?;
    check!(eq; contacts.len(), 49);
    check!(contacts.iter().all(|contact| matches!(
        contact.status,
        crate::restate_services::wire::ContactStatus::Exhausted
    )));
    Ok(())
}
