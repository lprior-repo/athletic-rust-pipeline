use census_domain::model::{ReviewState, ReviewVerdictRecord};
use census_store::{StoreError, Table};

use super::support::{client, options, state, Fixture, TestResult};
use crate::run_lanes;
#[test]
fn single_duplicate_and_extra_lanes_are_explicitly_nonaccepting() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, _) = Fixture::school()?;
            let first = client("http://127.0.0.1:18081")?;
            let second = client("http://127.0.0.1:18082")?;
            let duplicate = client("http://127.0.0.2:18081/")?;
            for clients in [
                vec![],
                vec![first.clone()],
                vec![first.clone(), duplicate],
                vec![
                    first.clone(),
                    second.clone(),
                    client("http://127.0.0.1:18083")?,
                ],
            ] {
                let error = match run_lanes(&fixture.store, &clients, &options(), "invalid").await {
                    Err(error) => error,
                    Ok(_) => return Err("two distinct lanes required".into()),
                };
                check!(matches!(error, StoreError::Invariant { .. }));
            }
            check!(matches!(
                run_lanes(
                    &fixture.store,
                    std::slice::from_ref(&first),
                    &options(),
                    "single"
                )
                .await,
                Err(StoreError::Invariant { .. })
            ));
            check!(eq; state(&fixture.store)?, ReviewState::Pending);
            check!(fixture
                .store
                .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?
                .is_empty());
            check!(eq; fixture.store.receipt_count()?, 0);
            Ok(())
        })
}
