use super::ledger::Attempt;
use crate::census::access::combined_failure;
use crate::restate_services::jobs::collect_error;
use crate::restate_services::wire::StageOutcome;
use crate::restate_services::JobError;
use census_crawl::{CollectionDisposition, CrawlError};

pub(super) fn failure(attempt: Attempt, secondary: CrawlError) -> Attempt {
    match attempt {
        Attempt::Completed(mut outcome) => {
            withhold(&mut outcome, &secondary);
            Attempt::Incomplete {
                outcome,
                error: collect_error(secondary),
            }
        }
        Attempt::Incomplete { mut outcome, error } => {
            withhold(&mut outcome, &secondary);
            Attempt::Incomplete {
                outcome,
                error: combined(error, &secondary),
            }
        }
        Attempt::Failed(primary) => Attempt::Failed(combined(primary, &secondary)),
    }
}

fn withhold(outcome: &mut StageOutcome, secondary: &CrawlError) {
    if outcome.disposition.is_complete() {
        outcome.disposition = CollectionDisposition::Partial;
    }
    outcome.errors.push(format!(
        "source access evidence could not be retained: {secondary}"
    ));
}

fn combined(primary: JobError, secondary: &CrawlError) -> JobError {
    let message = combined_failure(&primary, secondary).to_string();
    match primary {
        JobError::Transient { .. } => JobError::Transient { message },
        JobError::Terminal { .. } => JobError::Terminal { message },
        JobError::Resource { source, .. } => JobError::Resource { source, message },
    }
}
