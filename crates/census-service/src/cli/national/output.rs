use anyhow::Result;
use census_domain::national_census::NationalReport;
use census_domain::UsJurisdiction;
use census_reconcile::identity::WorkflowIdentity;
use restate_sdk::{
    ingress::{ClientError, Output},
    prelude::Json,
};
use std::collections::BTreeSet;

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct OutputBindingRefusal(&'static str);

pub(super) fn read_error(error: ClientError) -> anyhow::Error {
    let admission = census_crawl::ingress::is_admission_error(&error);
    let invalid = matches!(&error, ClientError::PayloadDecode { .. });
    let error = census_service::ingress::error(error);
    if admission {
        error.context(OutputBindingRefusal(
            "national attachment output exceeded transport admission",
        ))
    } else if invalid {
        error.context(OutputBindingRefusal(
            "national attachment output payload is invalid",
        ))
    } else {
        error
    }
}

pub(super) fn validate(
    headers: &reqwest::header::HeaderMap,
    output: &Output<Json<NationalReport>>,
    invocation_id: &str,
    identity: &WorkflowIdentity,
) -> Result<()> {
    if let Some(returned) = headers.get("x-restate-id") {
        let returned = returned.to_str().map_err(|_| {
            OutputBindingRefusal("national attachment output invocation ID is invalid")
        })?;
        if returned != invocation_id {
            return Err(OutputBindingRefusal(
                "national attachment output invocation ID differs from workflow lookup",
            )
            .into());
        }
    }
    if let Output::Ready(Json(report)) = output {
        validate_report(report, identity)?;
    }
    Ok(())
}

fn validate_report(report: &NationalReport, identity: &WorkflowIdentity) -> Result<()> {
    if report.is_partial() {
        return Err(OutputBindingRefusal(
            "national attachment output protocol refusal: Ready returned a Partial report",
        )
        .into());
    }
    let bound = UsJurisdiction::CENSUS_SCOPE.len();
    if report.jurisdictions.len() > bound
        || report.failures.len() > bound
        || report.owed.len() > bound
    {
        return Err(OutputBindingRefusal(
            "national attachment report exceeds the jurisdiction bound",
        )
        .into());
    }
    let scope: BTreeSet<_> = report
        .jurisdictions
        .iter()
        .map(|row| row.jurisdiction)
        .chain(report.failures.iter().map(|row| row.jurisdiction))
        .chain(report.owed.iter().map(|row| row.jurisdiction))
        .collect();
    if scope.is_empty() {
        return Err(
            OutputBindingRefusal("national attachment report has no jurisdiction scope").into(),
        );
    }
    let actual = WorkflowIdentity::national(
        report.season,
        report.revision,
        &scope.into_iter().collect::<Vec<_>>(),
    );
    let expected = identity
        .as_str()
        .rsplit_once(":resume:")
        .map_or(identity.as_str(), |(base, _)| base);
    if actual.as_str() != expected {
        return Err(OutputBindingRefusal("national attachment report differs from the requested season, revision or jurisdictions").into());
    }
    Ok(())
}
