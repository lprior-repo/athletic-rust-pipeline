use super::{
    build, check_selected_output, isolated, selected_fixture, selected_options, TestResult,
};
use census_report::export::ExportJobRequest;
use census_report::report::{ReportError, Scope};
use std::path::Path;

type RejectedRequest = (
    &'static str,
    fn(&mut census_report::workbook::Options) -> TestResult,
);

const REJECTED: [RejectedRequest; 5] = [
    ("rejected-cohort", |options| {
        options.grad_year = Some(2026);
        Ok(())
    }),
    ("rejected-jurisdictions", |options| {
        options.jurisdictions.clear();
        options
            .jurisdictions
            .push(census_domain::UsJurisdiction::Colorado);
        Ok(())
    }),
    ("rejected-school-year", |options| {
        options.school_year =
            census_domain::model::SchoolYear::new(2020).ok_or("invalid fixture school year")?;
        Ok(())
    }),
    ("rejected-source-scope", |options| {
        options.scope = Scope::Core;
        Ok(())
    }),
    ("rejected-zero-limit", |options| {
        options.limit = Some(0);
        Ok(())
    }),
];

#[test]
fn rejected_selected_request_does_not_reserve_a_stale_frozen_input() -> TestResult {
    if !isolated("selected_request_admission::rejected_selected_request_does_not_reserve_a_stale_frozen_input")? {
        return Ok(());
    }
    let controlled = selected_fixture::install()?;
    let request = ExportJobRequest::SelectedPublicationV1 {
        selection: controlled.selection()?,
    };
    let valid = selected_options(controlled.profile())?;
    for (job, change) in REJECTED {
        let mut unsupported = valid.clone();
        change(&mut unsupported)?;
        let error = build(controlled.store(), &unsupported, job, &request)
            .err()
            .ok_or_else(|| format!("invalid request {job} published successfully"))?;
        check!(
            matches!(error, ReportError::Invariant { .. }),
            "{job}: {error}"
        );
    }
    selected_fixture::revoke(&controlled)?;
    for (job, _) in REJECTED {
        let reply = build(controlled.store(), &valid, job, &request)?;
        check_selected_output(
            Path::new(&reply.path),
            controlled.public_id(),
            controlled.candidate_id(),
            controlled.current_school_id(),
            "51.00",
        )?;
        let verified =
            census_report::workbook::publication::verify_published(Path::new(&reply.path))?;
        check!(eq; verified.workbook.mapped_athletes, 1);
    }
    Ok(())
}
