use super::{
    closure::Closure, contexts::ResultContexts, invalid, note, ParticipationDiagnostic as Gap,
    PinnedParticipationRow,
};
use crate::export::{
    participation_bridge::{assess_with_independent, SourceCohortAgreement},
    ExportDataset,
};
use census_crawl::milesplit::{
    RetainedJournalView, RetainedResultSession, VerifiedRetainedResultMetadata,
};
use census_domain::model::{serialized_digest, CanonicalPerformance, Evidence};
use census_domain::participation_candidates::{
    CandidateOutcome, CandidateQuery, ParticipationReasons,
};
use census_store::StoreResult;
use serde_json::Value;
use std::collections::BTreeSet;

mod groups;
mod independent;
mod notes;

pub(super) struct Resolver<'a, V> {
    pub(super) dataset: &'a ExportDataset,
    pub(super) identities: crate::export::public_identity::population::IdentityPopulation<'a>,
    pub(super) sources: RetainedResultSession<'a, V>,
    pub(super) contexts: ResultContexts,
    pub(super) closure: &'a Closure,
    pub(super) roster_context: super::VerifiedRosterContext<'a>,
    pub(super) budget: super::budget::Captures,
    pub(super) independent:
        Option<&'a census_crawl::participation_evidence::IndependentParticipationBundleV1>,
}

impl<V: RetainedJournalView> Resolver<'_, V> {
    fn draft(
        &mut self,
        performance: &CanonicalPerformance,
        query: CandidateQuery<'_>,
    ) -> StoreResult<Accumulated> {
        let evidence = self.evidence(performance)?;
        let cohort = self.closure.cohort(&self.identities, performance);
        let mut result = Accumulated::default();
        self.closure
            .references(&self.identities, performance, &mut result.references)?;
        if !matches!(cohort, SourceCohortAgreement::Resolved(_)) {
            result.gaps.insert(Gap::CohortUnresolved);
        }
        let mut seen = BTreeSet::new();
        evidence
            .iter()
            .filter(|(_, value)| note::owned(value))
            .flat_map(|row| {
                evidence
                    .iter()
                    .filter(|(_, value)| note::metadata(value))
                    .map(move |meta| (row, meta))
            })
            .try_for_each(|(row, metadata)| {
                let Some(notes) = notes::read(row, metadata, &mut result, &mut seen)? else {
                    return Ok(());
                };
                self.assess(
                    performance,
                    query,
                    cohort,
                    &notes,
                    row,
                    metadata,
                    &mut result,
                )
            })?;
        if seen.is_empty() {
            result
                .gaps
                .insert(if performance.source_key.starts_with("milesplit_result:") {
                    Gap::MissingSourceProvenance
                } else {
                    Gap::UnsupportedProvider
                });
        }
        Ok(result)
    }

    #[cfg(test)]
    pub(super) fn row(
        &mut self,
        performance: &CanonicalPerformance,
        query: CandidateQuery<'_>,
    ) -> StoreResult<PinnedParticipationRow> {
        let draft = self.draft(performance, query)?;
        self.finish(performance, query, draft)
    }

    fn finish(
        &self,
        performance: &CanonicalPerformance,
        query: CandidateQuery<'_>,
        mut result: Accumulated,
    ) -> StoreResult<PinnedParticipationRow> {
        result.gaps.extend(self.closure.gate(
            self.dataset,
            &self.identities,
            performance,
            &self.contexts,
            self.roster_context,
        )?);
        result.finish(query, performance)
    }

    fn assess(
        &mut self,
        performance: &CanonicalPerformance,
        query: CandidateQuery<'_>,
        cohort: SourceCohortAgreement,
        notes: &note::SourceNotes,
        row: &(Evidence, Value),
        metadata: &(Evidence, Value),
        result: &mut Accumulated,
    ) -> StoreResult<()> {
        self.budget
            .charge(notes.owned.manifest_key(), notes.owned_capture.bytes)?;
        self.budget
            .charge(notes.raw.manifest_key(), notes.raw_capture.bytes)?;
        let source =
            match self
                .sources
                .read(&notes.reference, &notes.raw, &notes.owned, &notes.locator)
            {
                Ok(source) => source,
                Err(_) => {
                    result.gaps.insert(Gap::SourceUnavailable);
                    return Ok(());
                }
            };
        if !matches_note(&row.1, &metadata.1, notes, &source)
            || !super::context::matches(self.dataset, performance, &source, notes)
        {
            result.gaps.insert(Gap::SourceBindingMismatch);
            return Ok(());
        }
        let independent = self.independent_sources(performance, result)?;
        let assessment =
            match assess_with_independent(query, performance, &source, cohort, &independent) {
                Ok(assessment) => assessment,
                Err(_) => {
                    result.gaps.insert(Gap::SourceBindingMismatch);
                    return Ok(());
                }
            };
        self.contexts
            .retain_source(performance, &source, notes, &metadata.0)?;
        let (outcome, reasons, references) = assessment.into_parts();
        result.accept(outcome, reasons, references);
        Ok(())
    }

    fn evidence(&self, performance: &CanonicalPerformance) -> StoreResult<Vec<(Evidence, Value)>> {
        let mut seen = BTreeSet::new();
        self.closure
            .performances
            .iter()
            .filter(|row| row.id == performance.id)
            .flat_map(|row| &row.evidence)
            .try_fold(Vec::new(), |mut evidence, item| {
                if !seen.insert(serialized_digest(item)?) {
                    return Ok(evidence);
                }
                if seen.len() > 64 {
                    return Err(invalid("participation observation evidence exceeds 64"));
                }
                if let Some(value) = note::value(item)? {
                    evidence.push((item.clone(), value));
                }
                Ok(evidence)
            })
    }
}

#[derive(Default)]
struct Accumulated {
    outcome: Option<CandidateOutcome>,
    reasons: ParticipationReasons,
    references: BTreeSet<String>,
    gaps: BTreeSet<Gap>,
}

impl Accumulated {
    fn accept(
        &mut self,
        outcome: CandidateOutcome,
        reasons: ParticipationReasons,
        references: Vec<String>,
    ) {
        if self
            .outcome
            .is_some_and(|previous| previous != outcome || self.reasons != reasons)
        {
            self.gaps.insert(Gap::ConflictingSourceAssessments);
        }
        self.outcome = Some(outcome);
        self.reasons = self.reasons.union(reasons);
        self.references.extend(references);
    }

    fn finish(
        mut self,
        query: CandidateQuery<'_>,
        performance: &CanonicalPerformance,
    ) -> StoreResult<PinnedParticipationRow> {
        if self.outcome.is_none() {
            let (outcome, reasons) = super::unresolved::assess(query, performance)?;
            self.outcome = Some(outcome);
            self.reasons = self.reasons.union(reasons);
        }
        let outcome = match self.outcome {
            Some(CandidateOutcome::EligibleCandidate) if self.gaps.is_empty() => {
                CandidateOutcome::EligibleCandidate
            }
            Some(CandidateOutcome::Excluded) if self.gaps.is_empty() => CandidateOutcome::Excluded,
            _ => CandidateOutcome::Unresolved,
        };
        Ok(PinnedParticipationRow {
            outcome,
            reasons: self.reasons,
            references: self.references.into_iter().collect(),
            diagnostics: self.gaps.into_iter().collect(),
        })
    }
}

fn matches_note(
    row_note: &Value,
    meta_note: &Value,
    notes: &note::SourceNotes,
    source: &VerifiedRetainedResultMetadata,
) -> bool {
    let page = source.page();
    *row_note == super::support::owned_note(source, notes, source.result().row())
        && super::support::known_provider(&source.result().row().provider)
        && capture_matches(source.capture(), &notes.raw_capture)
        && capture_matches(
            source.result().acquisition().capture(),
            &notes.owned_capture,
        )
        && meta_note.get("meet_name") == Some(&serde_json::json!(page.meet.name))
        && meta_note.get("meet_date") == Some(&serde_json::json!(page.meet.date))
        && meta_note.get("region") == Some(&serde_json::json!(page.region))
        && meta_note.get("sport") == Some(&serde_json::json!(page.sport))
        && meta_note.get("school_year") == Some(&serde_json::json!(page.school_year))
}

fn capture_matches(
    actual: &census_crawl::net::FetchOutcome,
    note: &census_crawl::net::FetchOutcome,
) -> bool {
    actual.url == note.url
        && actual.response_url == note.response_url
        && actual.method == note.method
        && actual.status == note.status
        && actual.content_digest == note.content_digest
        && actual.bytes == note.bytes
        && actual.fetched_at == note.fetched_at
        && actual.content_type == note.content_type
}
