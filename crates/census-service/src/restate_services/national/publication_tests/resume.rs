use super::*;

pub(super) async fn run_scenario(scenario: Scenario, url: &str) -> TestResult {
    let client = reqwest::Client::builder()
        .http2_prior_knowledge()
        .timeout(Duration::from_secs(5))
        .build()?;
    let mut request = fixtures::request()?;
    if matches!(
        scenario,
        Scenario::ManagedComplete
            | Scenario::ManagedFailedStage
            | Scenario::ManagedRefusedSeal
            | Scenario::ManagedCompletedResume
            | Scenario::ManagedExhaustedResume
    ) {
        request.publication_contract =
            crate::restate_services::PublicationContract::ManagedBundleV2;
    }
    if matches!(scenario, Scenario::SelectedFailedStage) {
        request.publication_contract =
            crate::restate_services::PublicationContract::SelectedPublicationV1;
        request.export_job = census_report::export::ExportJobRequest::SelectedPublicationV1 {
            selection: census_domain::model::ProfileSelectionV1::new(
                census_domain::model::StoreIdentity::parse(&"a".repeat(64))?,
                census_domain::model::ProfileId::parse(&"b".repeat(64))?,
            ),
        };
    }
    let mut journal = journal::Journal::new(&request)?;
    match scenario {
        Scenario::ExternalControl => return super::external::local_control(&client, url).await,
        Scenario::Address(mode) => return binding::run(mode, &client, url).await,
        Scenario::FailedStage(stage) => journal.hold_stage = Some(stage.to_owned()),
        Scenario::RefusedSeal => journal.hold_stage = Some("Census".to_owned()),
        Scenario::PartialContacts => journal.partial_contacts = true,
        Scenario::UnfinishedRosters => journal.unfinished_rosters = true,
        Scenario::Complete => {}
        Scenario::ManagedComplete => {}
        Scenario::ManagedFailedStage => journal.hold_stage = Some("Workbook".to_owned()),
        Scenario::SelectedFailedStage => journal.hold_stage = Some("Workbook".to_owned()),
        Scenario::ManagedRefusedSeal => journal.hold_stage = Some("Census".to_owned()),
        Scenario::CompletedResume | Scenario::ManagedCompletedResume => {
            journal.unfinished_rosters = true
        }
        Scenario::ExhaustedResume | Scenario::ManagedExhaustedResume => {
            journal.partial_contacts = true
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
    }
    journal.drive(&client, url).await?;
    let retained = tempfile::tempdir()?;
    if matches!(
        scenario,
        Scenario::CompletedResume
            | Scenario::ExhaustedResume
            | Scenario::ManagedCompletedResume
            | Scenario::ManagedExhaustedResume
    ) {
        let original_calls = journal.calls.clone();
        check!(eq; publication_services(&journal)?, Vec::<String>::new());
        check!(journal.output.is_some());
        let source_store = if matches!(
            scenario,
            Scenario::ExhaustedResume | Scenario::ManagedExhaustedResume
        ) {
            let store = census_store::Store::open(retained.path())?;
            for call in &original_calls {
                if call.text(1)? == "TeamsSource" && call.text(2)? == "contacts" {
                    let request = serde_json::from_slice(call.bytes(3)?)?;
                    crate::restate_services::jurisdiction::team_source::tests::national_resume::exhaust(
                        &store, &call.text(5)?, &request,
                    )?;
                }
            }
            store.flush()?;
            drop(store);
            Some(Arc::new(census_store::Store::open(retained.path())?))
        } else {
            None
        };
        journal = journal::Journal::new(&request)?;
        journal.identity.push_str(":resume:1");
        journal.invocation = "invocation-national-resume-fixture".to_owned();
        journal.source_store = source_store;
        journal.drive(&client, url).await?;
        let source_calls = |calls: &[codec::Frame]| -> TestResult<Vec<(String, Vec<u8>)>> {
            calls
                .iter()
                .filter(|call| {
                    call.text(1).is_ok_and(|service| {
                        service == "JurisdictionCensus" || service == "TeamsSource"
                    })
                })
                .map(|call| Ok((call.text(5)?, call.bytes(3)?.to_vec())))
                .collect()
        };
        check!(eq; source_calls(&journal.calls)?, source_calls(&original_calls)?);
        let calls = journal.calls.clone();
        journal.step(&client, url).await?;
        check!(eq; journal.calls, calls);
    }
    match scenario {
        Scenario::FailedStage(stage) => resume_stage(&mut journal, &client, url, stage).await?,
        Scenario::RefusedSeal => refresh_seal(&mut journal, &client, url).await?,
        Scenario::ManagedFailedStage => {
            resume_stage(&mut journal, &client, url, "Workbook").await?
        }
        Scenario::SelectedFailedStage => {
            resume_stage(&mut journal, &client, url, "Workbook").await?
        }
        Scenario::ManagedRefusedSeal => refresh_seal(&mut journal, &client, url).await?,
        Scenario::ExhaustedResume | Scenario::ManagedExhaustedResume => {
            check!(eq; publication_services(&journal)?, Vec::<String>::new());
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
            return Ok(());
        }
        Scenario::PartialContacts | Scenario::UnfinishedRosters => {
            check!(eq; publication_services(&journal)?, Vec::<String>::new());
            check!(eq; journal.output.as_ref().and_then(|row| row.get("jurisdictions")).and_then(serde_json::Value::as_array).map(Vec::len), Some(49));
            return Ok(());
        }
        Scenario::InvalidResume(_) => return Err("invalid resume reached completion".into()),
        Scenario::Address(_) => {
            return Err("address scenario reached publication assertions".into())
        }
        Scenario::ExternalControl => {
            return Err("external control reached publication assertions".into())
        }
        Scenario::Complete
        | Scenario::CompletedResume
        | Scenario::ManagedComplete
        | Scenario::ManagedCompletedResume => {}
    }
    assert_published(&journal)
}
