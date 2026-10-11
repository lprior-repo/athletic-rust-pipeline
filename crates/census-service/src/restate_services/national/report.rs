use census_domain::model::SchoolYear;
use census_reconcile::identity::{Revision, WorkflowIdentity};
use restate_sdk::prelude::*;

use crate::restate_services::school_address_join::SchoolAddressJoinClient;
use crate::restate_services::wire::{
    JurisdictionOwed, JurisdictionSummary, NationalFailure, NationalReport, NationalRequest,
    SchoolAddressJoinReply,
};

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
) -> NationalReport {
    jurisdictions.sort_by_key(|summary| summary.jurisdiction.code());
    failures.sort_by_key(|failure| failure.jurisdiction.code());
    owed.sort_by_key(|owed| owed.jurisdiction.code());
    NationalReport {
        season,
        revision,
        rosters_total: jurisdictions
            .iter()
            .map(|summary| summary.rosters_total)
            .sum(),
        athletes_total: jurisdictions.iter().map(|summary| summary.athletes).sum(),
        class_of_2027_total: jurisdictions
            .iter()
            .map(|summary| summary.class_of_2027)
            .sum(),
        jurisdictions,
        failures,
        owed,
        school_address,
        today,
    }
}
