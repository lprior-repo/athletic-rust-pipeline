use crate::{
    config::Config,
    discovery::QueryPlan,
    exhaustive_ai::Analysis,
    exhaustive_identity as identity,
    exhaustive_search::search::{Discovery, StageError},
    extract, fetch,
    model::{Candidate, MatchRecord, ModelDecision, Prospect, SearchHit},
    scoring,
};
use anyhow::{Context, Result};
use std::{collections::BTreeMap, path::Path};

pub struct Engine {
    config: Config,
    discovery: Discovery,
    analysis: Option<Analysis>,
}

#[derive(Default)]
struct Evidence {
    hits: BTreeMap<String, SearchHit>,
    candidates: Vec<Candidate>,
    deterministic: Option<ModelDecision>,
}

pub(crate) fn merge_hit(hits: &mut BTreeMap<String, SearchHit>, hit: SearchHit) {
    match hits.get_mut(&hit.url) {
        Some(existing) => {
            if existing.title.is_empty() {
                existing.title = hit.title;
            }
            if !hit.snippet.is_empty() && !existing.snippet.contains(&hit.snippet) {
                existing.snippet.push('\n');
                existing.snippet.push_str(&hit.snippet);
            }
        }
        None => {
            hits.insert(hit.url.clone(), hit);
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum RowFailure {
    #[error("search failed: {0}")]
    Search(String),
    #[error("AI analysis failed: {0:#}")]
    Analysis(anyhow::Error),
    #[error("profile retrieval failed: {0:#}")]
    Retrieval(anyhow::Error),
    #[error("storage failed: {0:#}")]
    Storage(anyhow::Error),
}

impl RowFailure {
    fn status(&self) -> &'static str {
        match self {
            Self::Search(_) | Self::Retrieval(_) => "SEARCH_ERROR",
            Self::Analysis(_) => "AI_ERROR",
            Self::Storage(_) => "STORAGE_ERROR",
        }
    }
}

impl Engine {
    pub fn new(config: Config, out_dir: &Path, no_ai: bool) -> Result<Self> {
        let discovery = Discovery::new(&config, out_dir).context("initializing discovery")?;
        let analysis = if no_ai {
            None
        } else {
            Some(Analysis::new(&config, out_dir).context("initializing analysis")?)
        };
        Ok(Self {
            config,
            discovery,
            analysis,
        })
    }

    pub async fn process(&mut self, prospect: &Prospect) -> Result<MatchRecord> {
        if prospect.full_name().trim().is_empty() {
            return Ok(identity::error_record(
                prospect,
                Vec::new(),
                "INPUT_ERROR",
                "Missing name".to_owned(),
            ));
        }
        let mut evidence = Evidence::default();
        let outcome = self.analyze(prospect, &mut evidence).await;
        let mut record = match outcome {
            Ok(decision) => identity::finalize(
                prospect.clone(),
                evidence.candidates,
                decision,
                &self.config.matching,
                self.config.discovery.ambiguity_margin,
            ),
            Err(RowFailure::Storage(error)) => return Err(error),
            Err(error) => identity::error_record(
                prospect,
                evidence.candidates,
                error.status(),
                error.to_string(),
            ),
        };
        record.deterministic_decision = evidence.deterministic;
        Ok(record)
    }

    async fn analyze(
        &mut self,
        prospect: &Prospect,
        evidence: &mut Evidence,
    ) -> Result<ModelDecision, RowFailure> {
        // Complete every deduplicated query stage, not just the first plausible identity.
        let discovery = self.discover(prospect, &mut evidence.hits).await;
        evidence.candidates = evidence
            .hits
            .values()
            .map(|hit| {
                let mut candidate = extract::candidate_from_evidence_deterministic(
                    prospect,
                    hit,
                    None,
                    self.config.retrieval.page_text_limit,
                );
                scoring::score_athletics_candidate(prospect, &mut candidate, &self.config.matching);
                candidate
            })
            .collect();
        discovery?;
        let pages = self
            .enrich_deterministic(prospect, &mut evidence.candidates)
            .await?;
        let guess = identity::deterministic_decision(
            &evidence.candidates,
            &self.config.matching,
            self.config.discovery.ambiguity_margin,
        );
        evidence.deterministic = Some(guess.clone());
        let Some(analysis) = &mut self.analysis else {
            return Ok(guess);
        };
        if evidence.candidates.is_empty() {
            return Ok(guess);
        }
        // Fixed URL order makes model indexes reproducible regardless of HTTP completion order.
        for candidate in &mut evidence.candidates {
            let hit = evidence.hits.get(&candidate.profile_url).ok_or_else(|| {
                RowFailure::Storage(anyhow::anyhow!("candidate lost search provenance"))
            })?;
            let mut extracted = analysis
                .candidate(prospect, hit, pages.get(&hit.url).map(String::as_str))
                .await
                .map_err(RowFailure::Analysis)?;
            extracted.page_retrieved = candidate.page_retrieved;
            scoring::score_athletics_candidate(prospect, &mut extracted, &self.config.matching);
            *candidate = extracted;
        }
        analysis
            .decision(prospect, &evidence.candidates)
            .await
            .map_err(RowFailure::Analysis)
    }

    async fn discover(
        &mut self,
        prospect: &Prospect,
        hits: &mut BTreeMap<String, SearchHit>,
    ) -> Result<(), RowFailure> {
        let plan = QueryPlan::for_prospect(prospect);
        for stage in 0..3 {
            let Some(requests) = plan.stage(stage) else {
                continue;
            };
            match self.discovery.stage(requests).await {
                Ok(found) => found.into_iter().for_each(|hit| merge_hit(hits, hit)),
                Err(StageError::Search {
                    reason,
                    partial_hits,
                }) => {
                    partial_hits
                        .into_iter()
                        .for_each(|hit| merge_hit(hits, hit));
                    return Err(RowFailure::Search(reason));
                }
                Err(StageError::Storage(error)) => return Err(RowFailure::Storage(error)),
            }
        }
        Ok(())
    }

    async fn enrich_deterministic(
        &self,
        prospect: &Prospect,
        candidates: &mut [Candidate],
    ) -> Result<BTreeMap<String, String>, RowFailure> {
        let mut pages = BTreeMap::new();
        // Authorized/saved enrichment applies to every candidate, not only the first guess.
        for candidate in candidates {
            let Some(html) = self
                .profile_html(&candidate.profile_url)
                .await
                .map_err(RowFailure::Retrieval)?
            else {
                continue;
            };
            let hit = SearchHit {
                url: candidate.profile_url.clone(),
                title: candidate.search_title.clone(),
                snippet: candidate.search_snippet.clone(),
                ..Default::default()
            };
            *candidate = extract::candidate_from_evidence_deterministic(
                prospect,
                &hit,
                Some(&html),
                self.config.retrieval.page_text_limit,
            );
            scoring::score_athletics_candidate(prospect, candidate, &self.config.matching);
            pages.insert(candidate.profile_url.clone(), html);
        }
        Ok(pages)
    }

    async fn profile_html(&self, url: &str) -> Result<Option<String>> {
        if let Some(directory) = &self.config.retrieval.saved_pages_dir {
            if let Some(html) = fetch::load_saved_profile(url, directory)? {
                return Ok(Some(html));
            }
        }
        if self.config.retrieval.authorized_direct_fetch {
            return fetch::fetch_exact_profile(url, &self.config.retrieval)
                .await
                .map(Some);
        }
        Ok(None)
    }
}
