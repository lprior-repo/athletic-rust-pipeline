use super::Run;
use crate::CollectionDisposition;
use census_domain::model::ContactResearchOutcome;

impl Run<'_> {
    pub(super) fn finish_frontier(&mut self, expected_states: usize) {
        self.report
            .unfinished
            .extend(self.incomplete_states.iter().map(|state| {
                format!(
                    "{} has unresolved school/program contact research",
                    state.code()
                )
            }));
        self.report.unfinished.extend(
            self.wanted
                .difference(&self.seen_names)
                .map(|name| format!("requested school was not completed: {name}")),
        );
        if self.drained_states == expected_states {
            self.report.finish_frontier();
        } else {
            self.report.unfinished.push(format!(
                "{} of {expected_states} configured directory frontiers drained",
                self.drained_states
            ));
            if self.report.disposition == CollectionDisposition::Unknown {
                self.report.disposition = CollectionDisposition::Partial;
            }
        }
    }
}

pub(super) fn failure_disposition(outcome: ContactResearchOutcome) -> CollectionDisposition {
    match outcome {
        ContactResearchOutcome::Blocked => CollectionDisposition::Blocked,
        ContactResearchOutcome::Failed => CollectionDisposition::Failed,
        ContactResearchOutcome::Ambiguous | ContactResearchOutcome::Conflict => {
            CollectionDisposition::Quarantined
        }
        ContactResearchOutcome::Unattempted
        | ContactResearchOutcome::CompletedEmpty
        | ContactResearchOutcome::CompletedClaims
        | ContactResearchOutcome::Partial
        | ContactResearchOutcome::Exhausted
        | ContactResearchOutcome::Stale => CollectionDisposition::Partial,
    }
}
