use super::{fixtures, journal::Journal, TestResult};
use crate::restate_services::jobs::selected_fixture;
use crate::restate_services::ExportGeneration;
use census_report::report::ReportError;
use std::path::Path;

#[tokio::test]
async fn resumed_selected_generation_refuses_recapture_and_new_generation_selects_fallback(
) -> TestResult {
    super::exercise(super::Scenario::Generation).await
}

pub(super) async fn run(
    client: &reqwest::Client,
    url: &str,
    controlled: selected_fixture::Controlled,
) -> TestResult {
    let mut request =
        fixtures::selected_request(&controlled, &[census_domain::UsJurisdiction::Colorado])?;
    let mut first = Journal::new(&request)?;
    first.selected_workbook = Some(controlled);
    drive_one_invocation(&mut first, client, url).await?;
    verify(&first, "50.00")?;
    let original = Path::new(
        first
            .selected_workbook_path
            .as_deref()
            .ok_or("original workbook")?,
    );
    let original_bytes = std::fs::read(original)?;
    let mut unchanged = Journal::new(&request)?;
    unchanged.identity.push_str(":resume:1");
    unchanged.invocation = "invocation-generation-preserved".to_owned();
    unchanged.selected_workbook = first.selected_workbook.take();
    drive_one_invocation(&mut unchanged, client, url).await?;
    verify(&unchanged, "50.00")?;
    check!(eq; unchanged.selected_workbook_path.as_deref(), original.to_str());
    check!(eq; std::fs::read(original)?, original_bytes);
    first.selected_workbook = unchanged.selected_workbook.take();
    selected_fixture::revoke(
        first
            .selected_workbook
            .as_ref()
            .ok_or("original controlled source")?,
    )
    .map_err(|error| -> Box<dyn std::error::Error> { error })?;
    let mut resumed = Journal::new(&request)?;
    resumed.identity.push_str(":resume:2");
    resumed.invocation = "invocation-generation-unchanged".to_owned();
    resumed.selected_workbook = first.selected_workbook.take();
    let result = drive_one_invocation(&mut resumed, client, url).await;
    if result.is_ok() {
        verify(&resumed, "50.00")?;
        return Err("withdrawn same-generation request completed instead of refusing".into());
    }
    let error = result.err().ok_or("same-generation refusal missing")?;
    check!(
        matches!(
            error.downcast_ref::<ReportError>(),
            Some(ReportError::Invariant { .. })
        ),
        "{error:?}"
    );
    check!(eq; resumed.selected_workbook_path, None);
    check!(!resumed.calls.iter().any(|call| {
        call.text(1).is_ok_and(|name| name == "Census")
            && call.text(2).is_ok_and(|name| name == "publication")
    }));
    request.generation = Some(ExportGeneration::parse("after-withdrawal")?);
    let mut next = Journal::new(&request)?;
    next.identity.push_str(":resume:3");
    next.invocation = "invocation-generation-after-withdrawal".to_owned();
    next.selected_workbook = resumed.selected_workbook.take();
    drive_one_invocation(&mut next, client, url).await?;
    verify(&next, "51.00")?;
    let next_path = Path::new(
        next.selected_workbook_path
            .as_deref()
            .ok_or("next workbook")?,
    );
    check!(original != next_path);
    check!(eq; std::fs::read(original)?, original_bytes);
    let controlled = next
        .selected_workbook
        .as_ref()
        .ok_or("retained controlled source")?;
    verify_artifact(original, controlled, "50.00")?;
    if std::env::var_os("CENSUS_CONTROLLED_THREE_MARKS_RETAIN").is_some() {
        let controlled = next
            .selected_workbook
            .take()
            .ok_or("controlled artifact owner")?;
        let retained = selected_fixture::retain(controlled);
        census_report::workbook::publication::verify_published(original)?;
        census_report::workbook::publication::verify_published(next_path)?;
        eprintln!("NATIONAL_GENERATION_G0={}", original.display());
        eprintln!("NATIONAL_GENERATION_G1={}", next_path.display());
        eprintln!("NATIONAL_GENERATION_SOURCE={}", retained.display());
    }
    Ok(())
}

async fn drive_one_invocation(
    journal: &mut Journal,
    client: &reqwest::Client,
    url: &str,
) -> TestResult {
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        journal.drive(client, url),
    )
    .await?
}

fn verify(journal: &Journal, mark: &str) -> TestResult {
    check!(journal.output.is_some());
    let path = Path::new(
        journal
            .selected_workbook_path
            .as_deref()
            .ok_or("published workbook")?,
    );
    let controlled = journal
        .selected_workbook
        .as_ref()
        .ok_or("controlled source")?;
    verify_artifact(path, controlled, mark)
}

fn verify_artifact(
    path: &Path,
    controlled: &selected_fixture::Controlled,
    mark: &str,
) -> TestResult {
    let verified = census_report::workbook::publication::verify_published(path)?;
    check!(eq; verified.workbook.mapped_athletes, 1);
    crate::restate_services::jobs::publication_tests::check_selected_output(
        path,
        controlled.public_id(),
        controlled.candidate_id(),
        controlled.current_school_id(),
        mark,
    )
    .map_err(|error| -> Box<dyn std::error::Error> { error })?;
    Ok(())
}
