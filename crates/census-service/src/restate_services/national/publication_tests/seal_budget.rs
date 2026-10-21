use super::{fixtures, journal::Journal, TestResult};

pub(super) async fn run(
    client: &reqwest::Client,
    url: &str,
    controlled: crate::restate_services::jobs::selected_fixture::Controlled,
) -> TestResult {
    let mut request =
        fixtures::selected_request(&controlled, &census_domain::UsJurisdiction::CENSUS_SCOPE)?;
    request.pass_budget = 0;
    request.pass_delay_seconds = 0;
    let mut journal = Journal::new(&request)?;
    journal.selected_workbook = Some(controlled);
    journal.hold_stage = Some("Census".to_owned());
    journal.drive(client, url).await?;
    let workbook = journal
        .selected_workbook_path
        .clone()
        .ok_or("selected workbook absent")?;
    let generation =
        census_report::workbook::publication::verify_published(std::path::Path::new(&workbook))?
            .generation_digest;
    journal.release_refusal()?;
    check!(journal.drive(client, url).await.is_err());
    let failure = journal
        .terminal_failure
        .as_ref()
        .ok_or("seal exhaustion was not terminal")?;
    check!(eq; failure.number(1)?, 409);
    check!(journal.output.is_none());
    let mut measurements = 0;
    let mut workbooks = 0;
    for call in &journal.calls {
        match (call.text(1)?.as_str(), call.text(2)?.as_str()) {
            ("Census", "publication") => measurements += 1,
            ("Workbook", "run") => workbooks += 1,
            _ => {}
        }
    }
    check!(eq; measurements, 1);
    check!(eq; workbooks, 1);
    check!(!journal.entries.iter().any(|entry| entry.kind == 0x040C));
    check!(eq;
        census_report::workbook::publication::verify_published(std::path::Path::new(&workbook))?.generation_digest,
        generation
    );
    Ok(())
}
