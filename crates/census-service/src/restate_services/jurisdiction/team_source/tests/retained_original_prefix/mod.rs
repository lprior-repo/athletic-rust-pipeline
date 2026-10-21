use super::{key, ledger};
use crate::restate_services::national::publication_tests::codec;
use census_store::{Store, Table};
use std::sync::atomic::Ordering;
use std::sync::Arc;

mod parents;
mod prefix;
mod transport;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
async fn pre_change_ihsa_original_journal_replays_to_the_same_pending_sleep_without_effects(
) -> TestResult {
    let prefix = prefix::read()?;
    let root = tempfile::tempdir()?;
    let store = Arc::new(Store::open(root.path())?);
    let clock = Arc::new(transport::UnusedClock::default());
    check!(eq; key(&prefix.request), prefix::KEY);
    check!(store.journal_keys(ledger::PHASE)?.is_empty());
    check!(store
        .scan::<census_domain::model::SourceAccessCondition>(Table::SourceAccess)?
        .is_empty());

    let frames = transport::exchange(&store, &clock, &prefix).await?;

    let mut expected = codec::blob(1, &[7])?;
    expected.extend(codec::blob(2, &[1])?);
    expected.extend(codec::integer(5, 1));
    check!(
        (1..=2).contains(&frames.len()),
        "unexpected SDK output: {frames:?}"
    );
    let (suspension, updates) = frames.split_last().ok_or("missing Sleep suspension")?;
    check!(eq; suspension.kind, 0x0001);
    check!(eq; suspension.payload, codec::blob(4, &expected)?);
    for update in updates {
        check!(eq; update.kind, 0x0006);
        check!(eq; update.payload, codec::blob(1, &expected)?);
    }
    check!(eq; clock.0.load(Ordering::SeqCst), 0);
    check!(store.journal_keys(ledger::PHASE)?.is_empty());
    check!(store
        .journal_keys(ledger::origin_continuation::PHASE)?
        .is_empty());
    check!(store
        .scan::<census_domain::model::SourceAccessCondition>(Table::SourceAccess)?
        .is_empty());
    let attempts = ledger::durable_attempts(&store)?;
    check!(eq; (attempts.sources, attempts.reserved, attempts.recorded), (0, 0, 0));
    Ok(())
}
