use super::{emit, open, SourceOutcome};
use anyhow::{Context, Result};
use census_domain::model::{tag, AthleteCandidateId, GradYear, IdTag};
use std::path::Path;

fn validate_candidate(value: &str) -> Result<AthleteCandidateId> {
    let candidate: AthleteCandidateId = serde::Deserialize::deserialize(
        serde::de::value::StrDeserializer::<serde::de::value::Error>::new(value),
    )
    .context("parsing the athlete candidate identifier")?;
    anyhow::ensure!(
        candidate.validate(tag::Athlete::PREFIX),
        "source athlete subject identifier differs"
    );
    Ok(candidate)
}

pub(super) fn establish(root: &Path, operation: &str, candidate: &str) -> Result<()> {
    let candidate = validate_candidate(candidate)?;
    let (store, writer) = open(root)?;
    let head = writer.head(&store)?;
    let revision = store.snapshot().revision();
    let outcome = writer
        .establish_source_bound(&store, operation, &candidate, revision, &head)
        .context("source-bound identity establishment failed; inspect the store for revision or head movement")?;
    emit(&SourceOutcome {
        schema: "local-identity-source",
        operation,
        subject: candidate.as_str(),
        public_id: outcome.public_id(),
        head: outcome.head(),
        replayed: outcome.replayed(),
        actor: "local_operator",
        qualification_scope: "Unauthenticated local-operator request for deterministic source-bound identity only; no cross-source merge, current-school affiliation, participation eligibility, workbook publication or national acceptance is conferred",
    })
}

pub(super) fn cohort(root: &Path, operation: &str, graduation_year: i16) -> Result<()> {
    let cohort = GradYear::new(graduation_year).context("invalid graduation year")?;
    let (store, writer) = open(root)?;
    let head = writer.head(&store)?;
    let revision = store.snapshot().revision();
    let outcomes = writer
        .establish_source_bound_cohort(
            &store,
            operation,
            cohort,
            revision,
            &head,
        )
        .context("source-bound cohort establishment failed; retained capture and original operation remain recoverable")?;
    emit(&serde_json::json!({
        "schema": "local-identity-source-cohort/v1",
        "operation": operation,
        "graduation_year": graduation_year,
        "accepted": outcomes.len(),
        "head": outcomes.last().map(|outcome| outcome.head()),
        "replayed": outcomes.iter().all(|outcome| outcome.replayed()),
        "qualification_scope": "Deterministic source-bound identity only; current affiliation, participation eligibility and workbook publication require their existing evidence",
    }))
}
