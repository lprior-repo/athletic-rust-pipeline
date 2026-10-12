use census_store::{Store, StoreResult};

use crate::review_intake::CorpusIndex;
use crate::{ModelClient, ReviewOptions, ReviewReport, CHECKPOINT_CASES};

#[tracing::instrument(skip_all)]
pub async fn run_lanes_after(
    store: &Store,
    clients: &[ModelClient],
    options: &ReviewOptions,
    observed_at: &str,
    after: Option<&str>,
) -> StoreResult<(ReviewReport, Option<String>)> {
    let clients = crate::consensus::independent(clients)?;
    if options.limit > CHECKPOINT_CASES {
        return Err(crate::consensus::invariant(format!(
            "review checkpoint limit {} exceeds {CHECKPOINT_CASES}",
            options.limit
        )));
    }
    if options.limit == 0 {
        return Ok((ReviewReport::default(), None));
    }
    let original = store.snapshot();
    if after.is_none() && !options.dry_run {
        crate::revocation::invalidate(store, options, clients, observed_at).await?;
    }
    let mut corpus = CorpusIndex::new(&original);
    let intake = crate::review_intake::next(store, &mut corpus, options, after)?;
    let count = intake.cases.len().min(options.limit);
    let selected = intake
        .cases
        .get(..count)
        .ok_or_else(|| crate::consensus::invariant("review checkpoint selection disappeared"))?;
    let cursor = if count < intake.cases.len() {
        selected.last().map(|(case, _)| case.id.clone())
    } else {
        intake.cursor
    };
    let mut report = ReviewReport::default();
    if !selected.is_empty() {
        crate::review_process::process(
            store,
            selected,
            clients,
            &original,
            options,
            crate::review_process::Checkpoint {
                observed_at,
                index: 0,
            },
            &mut report,
        )
        .await?;
    }
    Ok((report, cursor))
}
