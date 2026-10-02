use census_domain::model::{ReviewState, ReviewVerdictRecord};
use census_store::{StoreError, Table};

use super::support::{client, options, state, Fixture};
use crate::run_lanes;
#[tokio::test]
async fn single_duplicate_and_extra_lanes_are_explicitly_nonaccepting() {
    let (fixture, _) = Fixture::school();
    let first = client("http://127.0.0.1:18081");
    let second = client("http://127.0.0.1:18082");
    let duplicate = client("http://127.0.0.2:18081/");
    for clients in [
        vec![],
        vec![first.clone()],
        vec![first.clone(), duplicate],
        vec![
            first.clone(),
            second.clone(),
            client("http://127.0.0.1:18083"),
        ],
    ] {
        let error = run_lanes(&fixture.store, &clients, &options(), "invalid")
            .await
            .expect_err("two distinct lanes required");
        assert!(matches!(error, StoreError::Invariant { .. }));
    }
    assert!(matches!(
        run_lanes(
            &fixture.store,
            std::slice::from_ref(&first),
            &options(),
            "single"
        )
        .await,
        Err(StoreError::Invariant { .. })
    ));
    assert_eq!(state(&fixture.store), ReviewState::Pending);
    assert!(fixture
        .store
        .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)
        .expect("rows")
        .is_empty());
    assert_eq!(fixture.store.receipt_count().expect("receipt"), 0);
}
