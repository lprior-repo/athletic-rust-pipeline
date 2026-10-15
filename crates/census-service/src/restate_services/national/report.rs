use census_domain::model::{Revision, SchoolYear};
use census_domain::national_census::{
    JurisdictionOwed, JurisdictionSummary, NationalCompleteness, NationalFailure, NationalReport,
    NationalReportError, NationalTotals, SchoolAddressJoinReply,
};
use census_reconcile::identity::WorkflowIdentity;
use restate_sdk::prelude::*;

use crate::restate_services::school_address_join::SchoolAddressJoinClient;
use crate::restate_services::wire::NationalRequest;

pub(super) async fn join_addresses(
    ctx: &WorkflowContext<'_>,
    identity: &WorkflowIdentity,
    request: &NationalRequest,
) -> Result<Option<SchoolAddressJoinReply>, HandlerError> {
    let Some(school_address) = request.school_address.clone() else {
        return Ok(None);
    };
    let Json(join) = ctx
        .workflow_client::<SchoolAddressJoinClient>(format!(
            "{}:school-address-join",
            identity.as_str()
        ))
        .run(Json(school_address))
        .call()
        .await?;
    tracing::info!(
        linked = join.counters.linked,
        already_linked = join.counters.already_linked,
        review = join.counters.review,
        no_match = join.counters.no_match,
        evidence_missing = join.counters.evidence_missing,
        refused = join.counters.refused,
        "joined school postal addresses for the run"
    );
    Ok(Some(join))
}

pub(super) fn assemble(
    season: SchoolYear,
    revision: Revision,
    mut jurisdictions: Vec<JurisdictionSummary>,
    mut failures: Vec<NationalFailure>,
    mut owed: Vec<JurisdictionOwed>,
    school_address: Option<SchoolAddressJoinReply>,
    today: String,
) -> Result<NationalReport, NationalReportError> {
    let totals = NationalTotals::of(&jurisdictions)?;
    jurisdictions.sort_by_key(|summary| summary.jurisdiction.code());
    failures.sort_by_key(|failure| failure.jurisdiction.code());
    owed.sort_by_key(|owed| owed.jurisdiction.code());
    Ok(NationalReport {
        season,
        revision,
        rosters_total: totals.rosters,
        athletes_total: totals.athletes,
        class_of_2027_total: totals.class_of_2027,
        jurisdictions,
        failures,
        owed,
        school_address,
        today,
        completeness: NationalCompleteness::Final,
    })
}

pub(super) fn assemble_partial(
    season: SchoolYear,
    revision: Revision,
    jurisdictions: &[JurisdictionSummary],
    failures: &[NationalFailure],
    owed: &[JurisdictionOwed],
) -> Result<NationalReport, NationalReportError> {
    let mut report = assemble(
        season,
        revision,
        jurisdictions.to_vec(),
        failures.to_vec(),
        owed.to_vec(),
        None,
        String::new(),
    )?;
    report.completeness = NationalCompleteness::Partial;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use census_domain::UsJurisdiction;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn summary(jurisdiction: UsJurisdiction, rosters: usize) -> JurisdictionSummary {
        JurisdictionSummary {
            jurisdiction,
            identity: jurisdiction.code().to_string(),
            rosters_total: rosters,
            rosters_committed: rosters,
            rosters_remaining: 0,
            rosters_skipped: 0,
            blocked: false,
            athletes: rosters.saturating_mul(2),
            class_of_2027: rosters,
        }
    }

    fn season() -> Result<SchoolYear, Box<dyn std::error::Error>> {
        SchoolYear::new(2026).ok_or("invalid test season".into())
    }

    #[test]
    fn missing_completeness_decodes_as_final() -> TestResult {
        let report: NationalReport = serde_json::from_value(serde_json::json!({
            "season": 2026, "revision": 1,
            "jurisdictions": [], "failures": [],
            "rosters_total": 0, "athletes_total": 0, "class_of_2027_total": 0,
            "today": "2026-10-12"
        }))?;
        check!(eq; report.completeness, NationalCompleteness::Final);
        check!(!report.is_partial());
        check!(eq; report.completeness_label(), "final");
        Ok(())
    }

    #[test]
    fn partial_snapshot_is_observable_before_completion() -> TestResult {
        let first = summary(UsJurisdiction::Wisconsin, 7);
        let second = summary(UsJurisdiction::Iowa, 5);
        let partial = assemble_partial(
            season()?,
            Revision(1),
            std::slice::from_ref(&first),
            &[],
            &[],
        )?;
        check!(partial.is_partial());
        check!(eq; partial.completeness_label(), "partial");
        check!(eq; partial.jurisdictions.len(), 1);
        check!(eq; partial.rosters_total, 7);
        check!(eq; partial.athletes_total, 14);
        check!(eq; partial.class_of_2027_total, 7);
        let full = assemble_partial(season()?, Revision(1), &[first, second], &[], &[])?;
        check!(eq; full.jurisdictions.len(), 2);
        check!(eq; full.rosters_total, 12);
        check!(eq; full.athletes_total, 24);
        check!(eq; full.class_of_2027_total, 12);
        Ok(())
    }

    #[test]
    fn partial_counters_converge_to_final_on_completion() -> TestResult {
        let jurisdictions = vec![
            summary(UsJurisdiction::Wisconsin, 7),
            summary(UsJurisdiction::Iowa, 5),
        ];
        let failures: Vec<NationalFailure> = Vec::new();
        let owed: Vec<JurisdictionOwed> = Vec::new();
        let partial = assemble_partial(season()?, Revision(1), &jurisdictions, &failures, &owed)?;
        let final_report = assemble(
            season()?,
            Revision(1),
            jurisdictions.clone(),
            failures.clone(),
            owed.clone(),
            None,
            "2026-10-12".to_string(),
        )?;
        check!(partial.is_partial());
        check!(!final_report.is_partial());
        check!(eq; final_report.completeness, NationalCompleteness::Final);
        check!(eq; partial.rosters_total, final_report.rosters_total);
        check!(eq; partial.athletes_total, final_report.athletes_total);
        check!(eq; partial.class_of_2027_total, final_report.class_of_2027_total);
        check!(eq; partial.jurisdictions.len(), final_report.jurisdictions.len());
        let encoded = serde_json::to_value(&partial)?;
        check!(eq; encoded.get("completeness"), Some(&serde_json::json!("partial")));
        let decoded: NationalReport = serde_json::from_value(encoded)?;
        check!(eq; decoded.completeness, NationalCompleteness::Partial);
        Ok(())
    }

    #[test]
    fn report_total_overflow_returns_the_typed_metric() -> TestResult {
        let result = assemble(
            season()?,
            Revision(1),
            vec![
                summary(UsJurisdiction::Wisconsin, usize::MAX),
                summary(UsJurisdiction::Iowa, 1),
            ],
            Vec::new(),
            Vec::new(),
            None,
            String::new(),
        );
        check!(matches!(
            result,
            Err(NationalReportError::TotalOverflow {
                metric: census_domain::national_census::NationalMetric::Rosters,
            })
        ));
        Ok(())
    }
}
