use super::fixtures;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) fn publication_services(journal: &super::journal::Journal) -> TestResult<Vec<String>> {
    journal
        .calls
        .iter()
        .filter(|call| {
            call.text(1).is_ok_and(|name| {
                matches!(
                    name.as_str(),
                    "Consolidate" | "Report" | "Bests" | "Workbook"
                )
            }) || call.text(2).is_ok_and(|name| name == "publication")
        })
        .map(|call| call.text(1))
        .collect()
}

pub(super) fn assert_published(journal: &super::journal::Journal) -> TestResult {
    let managed = journal.original.publication_contract
        != crate::restate_services::PublicationContract::LegacySeparateV1;
    verify_publication_stages(journal, managed)?;
    super::binding::assert_requests(journal, managed)?;
    verify_seal(journal)?;
    verify_report(journal)?;
    if journal.original.publication_contract
        == crate::restate_services::PublicationContract::SelectedPublicationV1
    {
        verify_selected_output(journal)?;
    }
    Ok(())
}

fn publication_stages(managed: bool) -> &'static [&'static str] {
    if managed {
        &["Consolidate", "Workbook"]
    } else {
        &["Consolidate", "Report", "Bests", "Workbook"]
    }
}

fn verify_publication_stages(journal: &super::journal::Journal, managed: bool) -> TestResult {
    let stages = publication_stages(managed);
    let mut expected = stages.to_vec();
    expected.push("Census");
    expected.extend((0..journal.seal_refusals).map(|_| "Census"));
    check!(eq; publication_services(journal)?, expected);
    for &stage in stages {
        let call = journal
            .calls
            .iter()
            .find(|call| call.text(1).is_ok_and(|name| name == stage))
            .ok_or("publication stage absent")?;
        check!(eq; journal.calls.iter().filter(|call| call.text(1).is_ok_and(|name| name == stage)).count(), 1);
        check!(eq; call.text(5)?, format!("{}:{}", journal.identity, stage.to_ascii_lowercase()));
    }
    if journal.original.publication_contract
        == crate::restate_services::PublicationContract::SelectedPublicationV1
    {
        verify_selected_workbook_request(journal)?;
    }
    Ok(())
}

fn verify_selected_workbook_request(journal: &super::journal::Journal) -> TestResult {
    let workbook = journal
        .calls
        .iter()
        .find(|call| call.text(1).is_ok_and(|name| name == "Workbook"))
        .ok_or("selected publication did not call Workbook")?;
    let body: serde_json::Value = serde_json::from_slice(workbook.bytes(3)?)?;
    check!(eq;
        body.get("export_job"),
        Some(&serde_json::to_value(&journal.original.export_job)?),
        "the immutable profile selection reaches the workbook handler"
    );
    Ok(())
}

fn verify_seal(journal: &super::journal::Journal) -> TestResult {
    let seal = journal
        .calls
        .iter()
        .find(|call| call.text(2).is_ok_and(|handler| handler == "publication"))
        .ok_or("national never called the journal-measuring publication handler")?;
    check!(eq; seal.text(6)?, format!("{}:seal:0", journal.identity));
    let workbook = workbook_path(journal)?;
    check!(eq; serde_json::from_slice::<serde_json::Value>(seal.bytes(3)?)?, serde_json::json!({
        "grad_year": 2027, "all_sources": true, "workbook": workbook,
        "write": true, "season": 2026, "revision": 1, "source_objects": [], "jurisdictions": journal.original.jurisdictions
    }));
    Ok(())
}

fn workbook_path(journal: &super::journal::Journal) -> TestResult<&str> {
    Ok(match journal.original.publication_contract {
        crate::restate_services::PublicationContract::SelectedPublicationV1 => journal
            .selected_workbook_path
            .as_deref()
            .ok_or("selected National dispatch did not run the controlled workbook job")?,
        crate::restate_services::PublicationContract::ManagedBundleV2 => fixtures::MANAGED_WORKBOOK,
        crate::restate_services::PublicationContract::LegacySeparateV1 => {
            "out/publication/current/workbook.xlsx"
        }
    })
}

fn verify_report(journal: &super::journal::Journal) -> TestResult {
    let report = journal
        .output
        .as_ref()
        .ok_or("national did not return its completed report")?;
    let count = census_reconcile::identity::admitted_scope(&journal.original.jurisdictions).len();
    check!(eq; report.get("jurisdictions").and_then(serde_json::Value::as_array).map(Vec::len), Some(count));
    check!(eq; report.get("failures"), Some(&serde_json::json!([])));
    check!(eq; report.get("owed"), Some(&serde_json::json!([])));
    Ok(())
}

fn verify_selected_output(journal: &super::journal::Journal) -> TestResult {
    let controlled = journal
        .selected_workbook
        .as_ref()
        .ok_or("selected National controlled fixture missing")?;
    let path = journal
        .selected_workbook_path
        .as_ref()
        .ok_or("selected National workbook output missing")?;
    let verified =
        census_report::workbook::publication::verify_published(std::path::Path::new(path))?;
    check!(verified.workbook.mapped_athletes > 0);
    crate::restate_services::jobs::publication_tests::check_selected_output(
        std::path::Path::new(path),
        controlled.public_id(),
        controlled.candidate_id(),
        controlled.current_school_id(),
        "50.00",
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}
