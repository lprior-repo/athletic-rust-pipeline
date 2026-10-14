use super::JurisdictionCensus;
use crate::restate_services::wire::{
    AthleticLiveManifestRequest, AthleticLiveObligation, JurisdictionRequest, JurisdictionState,
    StageOutcome,
};
use crate::restate_services::{blocking, job_error, JobError};
use census_crawl::{net::Fetcher, AdapterContext, CollectionDisposition};
use census_store::Store;
use restate_sdk::prelude::*;
use std::sync::Arc;

pub(super) mod ledger;
#[cfg(test)]
mod tests;

impl JurisdictionCensus {
    pub(super) async fn bind_athleticlive(
        &self,
        ctx: &ObjectContext<'_>,
        request: &JurisdictionRequest,
        state: &mut JurisdictionState,
        today: &str,
    ) -> Result<(), HandlerError> {
        let store = Arc::clone(&self.store);
        let request = request.clone();
        state.athleticlive = blocking(self.region(), move || ledger::bind(&store, &request))
            .await
            .map_err(job_error)?;
        bind_plan(state);
        self.save(ctx, state, today);
        Ok(())
    }

    pub(super) async fn register_athleticlive(
        &self,
        ctx: &ObjectContext<'_>,
        request: AthleticLiveManifestRequest,
    ) -> Result<AthleticLiveObligation, HandlerError> {
        self.require_bound_run(&request.jurisdiction)?;
        if ctx.key()
            != census_reconcile::identity::WorkflowIdentity::jurisdiction(
                request.jurisdiction.jurisdiction,
                request.jurisdiction.season,
                request.jurisdiction.revision,
            )
            .as_str()
        {
            return Err(TerminalError::new(
                "AthleticLIVE manifest request does not match jurisdiction object key",
            )
            .into());
        }
        let store = Arc::clone(&self.store);
        let scope = request.jurisdiction.clone();
        let input = request.input;
        let today = crate::restate_services::journaled_today(ctx, &self.clock).await?;
        let at = today.clone();
        let outcome = blocking(self.region(), move || register(&store, &scope, &input, &at))
            .await
            .map_err(job_error)?;
        let mut state = self.load_object(ctx).await?;
        state.athleticlive = Some(outcome.clone());
        bind_plan(&mut state);
        self.save(ctx, &state, &today);
        Ok(outcome)
    }

    pub(super) async fn athleticlive_owed(
        &self,
        ctx: &ObjectContext<'_>,
        request: &JurisdictionRequest,
        state: &mut JurisdictionState,
        today: &str,
    ) -> Result<(), HandlerError> {
        if !state
            .athleticlive
            .as_ref()
            .is_some_and(|lane| lane.digest().is_some() && !lane.is_terminal())
        {
            return Ok(());
        }
        let store = Arc::clone(&self.store);
        let region = self.region();
        let request = request.clone();
        let at = today.to_string();
        let Json(outcome) = ctx
            .run(move || async move {
                blocking(region, move || execute(&store, &request, &at))
                    .await
                    .map(Json)
                    .map_err(job_error)
            })
            .await?;
        state.athleticlive = outcome;
        self.save(ctx, state, today);
        Ok(())
    }
}

pub(super) fn register(
    store: &Store,
    request: &JurisdictionRequest,
    input: &str,
    at: &str,
) -> Result<AthleticLiveObligation, JobError> {
    if !ledger::applicable(request) {
        return Err(ledger::error(
            "AthleticLIVE is not applicable to this jurisdiction",
        ));
    }
    let fetcher = offline_fetcher(store)?;
    let context = context(store, &fetcher, request, at)?;
    match census_crawl::athleticlive::freeze_manifest(&context, input) {
        Ok(frozen) => ledger::register(store, request, frozen),
        Err(error) => ledger::refuse(
            store,
            request,
            format!(
                "{}: retained AthleticLIVE manifest cannot qualify: {error}",
                request.jurisdiction.code()
            ),
        ),
    }
}

pub(super) fn execute(
    store: &Store,
    request: &JurisdictionRequest,
    at: &str,
) -> Result<Option<AthleticLiveObligation>, JobError> {
    let Some(reserved) = ledger::begin(store, request)? else {
        return ledger::status(store, request);
    };
    let manifest = ledger::manifest(store, request)?;
    if manifest.digest != reserved.0 {
        return Err(ledger::error(
            "AthleticLIVE frozen manifest differs from reserved digest",
        ));
    }
    let fetcher = offline_fetcher(store)?;
    let context = context(store, &fetcher, request, at)?;
    let options = census_crawl::athleticlive::ManifestOptions {
        observed_on: at.into(),
        states: vec![request.jurisdiction],
        ..Default::default()
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|source| ledger::error(&source.to_string()))?;
    let result = runtime.block_on(census_crawl::athleticlive::collect_manifest_body(
        &context,
        &options,
        &ledger::key(request),
        &manifest.body,
    ));
    let outcome = match result {
        Ok(report) => StageOutcome {
            records: usize::try_from(report.rows)
                .map_err(|_| ledger::error("AthleticLIVE row counter overflow"))?,
            at: at.into(),
            errors: if report.errors > 0 || report.rejections > 0 {
                vec![format!(
                    "{} errors, {} rejected; unresolved {:?}",
                    report.errors, report.rejections, report.unresolved
                )]
            } else {
                Vec::new()
            },
            notes: report.notes,
            disposition: report.disposition,
            unfinished: report.unfinished,
        },
        Err(error) => StageOutcome {
            records: 0,
            at: at.into(),
            errors: vec![error.to_string()],
            notes: Vec::new(),
            disposition: CollectionDisposition::Failed,
            unfinished: vec![ledger::key(request)],
        },
    };
    ledger::finish(store, request, reserved, outcome).map(Some)
}

fn offline_fetcher(store: &Store) -> Result<Fetcher, JobError> {
    Fetcher::new(
        store.http_cache_dir(),
        None,
        std::time::Duration::from_millis(1),
        std::collections::HashMap::new(),
        Vec::new(),
    )
    .map_err(|source| ledger::error(&source.to_string()))
}

fn context<'a>(
    store: &'a Store,
    fetcher: &'a Fetcher,
    request: &JurisdictionRequest,
    at: &str,
) -> Result<AdapterContext<'a>, JobError> {
    Ok(AdapterContext {
        capture: census_crawl::CaptureContext::LegacyMetadata,
        store,
        fetcher,
        refresh: false,
        school_year: request.season,
        observed_on: at.into(),
        performance_as_of: request.history.as_of(),
        recording: None,
    })
}

fn bind_plan(state: &mut JurisdictionState) {
    let Some(lane) = state.athleticlive.as_ref() else {
        return;
    };
    let Some(plan) = state.plan.as_mut() else {
        return;
    };
    plan.refused
        .retain(|refusal| refusal.slug != "athleticlive");
    if let AthleticLiveObligation::Refused { reason } = lane {
        plan.refused
            .push(crate::restate_services::wire::RefusedSource {
                slug: "athleticlive".into(),
                reason: reason.clone(),
                kind: crate::restate_services::plan::RefusalKind::EngineeringGap,
            });
    }
}
