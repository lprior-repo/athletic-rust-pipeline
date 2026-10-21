use super::{key, ledger, public_resources_fixture, request};
use crate::restate_services::jurisdiction::JurisdictionCensus;
use crate::restate_services::national::publication_tests::codec::{self, Frame};
use crate::restate_services::wire::{JurisdictionRequest, JurisdictionState};
use restate_sdk::prelude::*;

mod fixture;
mod full_pass;
mod journal;
mod verify;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[derive(Clone)]
struct OrdinaryPassSuffix {
    owner: JurisdictionCensus,
}

#[object]
impl OrdinaryPassSuffix {
    #[handler]
    async fn consume(
        &self,
        ctx: ObjectContext<'_>,
        Json(request): Json<JurisdictionRequest>,
    ) -> Result<Json<JurisdictionState>, HandlerError> {
        self.owner.require_bound_run(&request)?;
        let mut state = self.owner.load_object(&ctx).await?;
        self.owner
            .consume_team_continuation(&ctx, &request, &mut state)
            .await?;
        Ok(Json(state))
    }
}

#[tokio::test]
async fn native_suffix_consumes_after_rosters_and_preserves_other_source_failure() -> TestResult {
    verify::exercise("settled_failure").await
}

#[tokio::test]
async fn native_suffix_preserves_other_source_success_and_replays_commit_boundary() -> TestResult {
    verify::exercise("settled_success").await
}

#[tokio::test]
async fn native_suffix_retains_pending_and_repeated_terminal_continuation_budget() -> TestResult {
    verify::exercise("pending").await?;
    verify::exercise("continued_failure").await
}

#[tokio::test]
async fn native_suffix_refuses_mismatched_parent_owner_and_original_without_writes() -> TestResult {
    for mode in [
        "foreign_bound",
        "foreign_plan",
        "wrong_digest",
        "unrelated_parent",
        "unsettled_other",
    ] {
        verify::exercise(mode).await?;
    }
    Ok(())
}

#[tokio::test]
async fn native_suffix_refuses_non_ms_cross_disposition_overlaps_without_effects() -> TestResult {
    for mode in [
        "inventory_sweep_refused",
        "inventory_sweep_satisfied",
        "inventory_refused_satisfied",
    ] {
        verify::exercise(mode).await?;
    }
    Ok(())
}

#[tokio::test]
async fn native_suffix_refuses_non_ms_and_unselected_duplicates_without_effects() -> TestResult {
    for mode in [
        "inventory_sweep_duplicate",
        "inventory_unselected_duplicate",
        "inventory_refused_duplicate",
        "inventory_satisfied_duplicate",
    ] {
        verify::exercise(mode).await?;
    }
    Ok(())
}
