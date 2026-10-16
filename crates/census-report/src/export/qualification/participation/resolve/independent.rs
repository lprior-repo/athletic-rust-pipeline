use super::{Accumulated, Gap, Resolver};
use census_crawl::milesplit::RetainedJournalView;
use census_crawl::participation_evidence::VerifiedIndependentParticipationV1;
use census_domain::model::CanonicalPerformance;
use census_store::StoreResult;
use std::collections::BTreeSet;

impl<V: RetainedJournalView> Resolver<'_, V> {
    pub(super) fn independent_sources(
        &mut self,
        performance: &CanonicalPerformance,
        result: &mut Accumulated,
    ) -> StoreResult<Vec<VerifiedIndependentParticipationV1>> {
        let Some(bundle) = self.independent else {
            return Ok(Vec::new());
        };
        let evidence = self.evidence(performance)?;
        let mut seen = BTreeSet::new();
        evidence
            .iter()
            .try_fold(Vec::new(), |mut sources, (evidence, value)| {
                let Some(reference) = super::super::independent::reference(value)? else {
                    return Ok(sources);
                };
                result.references.insert(reference.as_str().into());
                if !seen.insert(reference.clone()) {
                    return Ok(sources);
                }
                match bundle.read(&reference) {
                    Ok(Some(source)) if super::super::independent::matches(evidence, &source) => {
                        [source.status_proof(), source.season_proof()]
                            .into_iter()
                            .for_each(|proof| {
                                result.references.extend([
                                    proof.capture().into(),
                                    proof.interpretation().into(),
                                    proof.locator().into(),
                                ]);
                            });
                        self.contexts
                            .retain_independent(&performance.id, evidence)?;
                        sources.push(source);
                    }
                    Ok(Some(_)) => {
                        result.gaps.insert(Gap::SourceBindingMismatch);
                    }
                    Ok(None) | Err(_) => {
                        result.gaps.insert(Gap::SourceUnavailable);
                    }
                }
                Ok(sources)
            })
    }
}
