use super::{
    model::{AthleticModelsClient, ExtractionJob, ProfileJob, ReviewJob},
    source::{AthleticSourceClient, SearchJob},
    step::{self, EffectFailure},
    Runtime,
};
use crate::{
    discovery::QueryPlan,
    exhaustive_identity as identity, extract,
    model::{Candidate, ModelDecision, SearchHit},
    restate_types::{row_key, RowInput},
    scoring,
};
use futures::{stream, StreamExt, TryStreamExt};
use restate_sdk::prelude::*;
use std::collections::BTreeMap;

pub(super) enum Failure {
    Execution(TerminalError),
    Domain(&'static str, EffectFailure),
}
impl From<TerminalError> for Failure {
    fn from(error: TerminalError) -> Self {
        Self::Execution(error)
    }
}
#[derive(Default)]
pub(super) struct Evidence {
    pub hits: BTreeMap<String, SearchHit>,
    pub candidates: Vec<Candidate>,
    pub deterministic: Option<ModelDecision>,
    failure: Option<Failure>,
}

pub(super) async fn analyze(
    ctx: &ObjectContext<'_>,
    runtime: &Runtime,
    input: &RowInput,
    evidence: &mut Evidence,
) -> Result<ModelDecision, Failure> {
    discover(ctx, input, evidence).await;
    evidence.candidates = evidence
        .hits
        .values()
        .map(|hit| {
            let mut candidate = extract::candidate_from_evidence_deterministic(
                &input.prospect,
                hit,
                None,
                runtime.config.retrieval.page_text_limit,
            );
            scoring::score_athletics_candidate(
                &input.prospect,
                &mut candidate,
                &runtime.config.matching,
            );
            candidate
        })
        .collect();
    if let Some(error) = evidence.failure.take() {
        return Err(error);
    }
    let pages = enrich(
        ctx,
        runtime,
        input,
        &evidence.hits,
        &mut evidence.candidates,
    )
    .await?;
    let guess = identity::deterministic_decision(
        &evidence.candidates,
        &runtime.config.matching,
        runtime.config.discovery.ambiguity_margin,
    );
    evidence.deterministic = Some(guess.clone());
    if input.no_ai || evidence.candidates.is_empty() {
        return Ok(guess);
    }
    extract_all(ctx, runtime, input, evidence, &pages).await?;
    review(ctx, input, &evidence.candidates).await
}

async fn discover(ctx: &ObjectContext<'_>, input: &RowInput, evidence: &mut Evidence) {
    let plan = QueryPlan::for_prospect(&input.prospect);
    let requests = (0..3)
        .filter_map(|stage| plan.stage(stage))
        .flatten()
        .cloned();
    stream::iter(requests)
        .fold(evidence, |state, request| async move {
            if state.failure.is_none() {
                let client = ctx.object_client::<AthleticSourceClient>("athletic-source");
                match client
                    .search(Json(SearchJob {
                        digest: input.config_digest.clone(),
                        namespace: input.run_fingerprint.clone(),
                        request,
                    }))
                    .call()
                    .await
                {
                    Ok(Json(Ok(hits))) => hits
                        .into_iter()
                        .for_each(|hit| crate::exhaustive_engine::merge_hit(&mut state.hits, hit)),
                    Ok(Json(Err(error))) => {
                        state.failure = Some(Failure::Domain("discovery", error))
                    }
                    Err(error) => state.failure = Some(Failure::Execution(error)),
                }
            }
            state
        })
        .await;
}

async fn enrich(
    ctx: &ObjectContext<'_>,
    runtime: &Runtime,
    input: &RowInput,
    hits: &BTreeMap<String, SearchHit>,
    candidates: &mut [Candidate],
) -> Result<BTreeMap<String, String>, Failure> {
    stream::iter(candidates.iter_mut().map(Ok::<_, Failure>))
        .try_fold(BTreeMap::new(), |mut pages, candidate| async move {
            let key = row_key("profile", &candidate.profile_url);
            let html = match step::cached::<Option<String>>(ctx, &key).await? {
                Some(html) => html,
                None => {
                    let client = ctx.object_client::<AthleticModelsClient>("retrieval");
                    let html = client
                        .profile(Json(ProfileJob {
                            digest: input.config_digest.clone(),
                            url: candidate.profile_url.clone(),
                            authorization_ack: input.authorization_ack,
                        }))
                        .call()
                        .await?
                        .0
                        .map_err(|error| Failure::Domain("retrieval", error))?;
                    ctx.set(&key, Json(html.clone()));
                    html
                }
            };
            if let Some(html) = html {
                let hit = hits.get(&candidate.profile_url).ok_or_else(|| {
                    Failure::Domain(
                        "retrieval",
                        EffectFailure::new(
                            "PROVENANCE",
                            "candidate lost discovery provenance",
                            false,
                        ),
                    )
                })?;
                *candidate = extract::candidate_from_evidence_deterministic(
                    &input.prospect,
                    hit,
                    Some(&html),
                    runtime.config.retrieval.page_text_limit,
                );
                scoring::score_athletics_candidate(
                    &input.prospect,
                    candidate,
                    &runtime.config.matching,
                );
                pages.insert(candidate.profile_url.clone(), html);
            }
            Ok(pages)
        })
        .await
}

async fn extract_all(
    ctx: &ObjectContext<'_>,
    runtime: &Runtime,
    input: &RowInput,
    evidence: &mut Evidence,
    pages: &BTreeMap<String, String>,
) -> Result<(), Failure> {
    let hits = &evidence.hits;
    stream::iter(evidence.candidates.iter_mut().map(Ok::<_, Failure>))
        .try_for_each(|candidate| async move {
            let hit = hits.get(&candidate.profile_url).ok_or_else(|| {
                Failure::Domain(
                    "extraction",
                    EffectFailure::new("PROVENANCE", "candidate lost discovery provenance", false),
                )
            })?;
            let key = row_key("extraction", &hit.url);
            let mut extracted = match step::cached::<Candidate>(ctx, &key).await? {
                Some(candidate) => candidate,
                None => {
                    let client = ctx.object_client::<AthleticModelsClient>("extractor");
                    let candidate = client
                        .extract(Json(ExtractionJob {
                            digest: input.config_digest.clone(),
                            prospect: input.prospect.clone(),
                            hit: hit.clone(),
                            html: pages.get(&hit.url).cloned(),
                        }))
                        .call()
                        .await?
                        .0
                        .map_err(|error| Failure::Domain("extraction", error))?;
                    ctx.set(&key, Json(candidate.clone()));
                    candidate
                }
            };
            extracted.page_retrieved = candidate.page_retrieved;
            scoring::score_athletics_candidate(
                &input.prospect,
                &mut extracted,
                &runtime.config.matching,
            );
            *candidate = extracted;
            Ok(())
        })
        .await
}

async fn review(
    ctx: &ObjectContext<'_>,
    input: &RowInput,
    candidates: &[Candidate],
) -> Result<ModelDecision, Failure> {
    if let Some(decision) = step::cached::<ModelDecision>(ctx, "review").await? {
        return Ok(decision);
    }
    let client = ctx.object_client::<AthleticModelsClient>("reviewer");
    let decision = client
        .review(Json(ReviewJob {
            digest: input.config_digest.clone(),
            prospect: input.prospect.clone(),
            candidates: candidates.to_vec(),
        }))
        .call()
        .await?
        .0
        .map_err(|error| Failure::Domain("review", error))?;
    ctx.set("review", Json(decision.clone()));
    Ok(decision)
}
