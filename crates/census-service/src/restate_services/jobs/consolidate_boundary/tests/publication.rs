use census_store::{Store, Table};

use super::super::{operation, reached, BoundaryError};
use super::{bind_evidence, consolidate_tables, marker_value, Arming, TestResult, KEY};

#[test]
fn consolidate_boundary_publication_failure_does_not_consume_the_replay_boundary() -> TestResult {
    let arming = Arming::clear();
    let directory = tempfile::tempdir()?;
    let parent = directory.path().join("boundary");
    let marker = parent.join("reached.json");
    let store = Store::open(directory.path().join("store"))?;
    let tables = [Table::Schools];
    let consolidated = consolidate_tables(&store, &tables)?;
    let operation = operation(KEY);
    arming.boundary(KEY, &marker, 1);

    match reached(&store, &operation, &consolidated) {
        Err(BoundaryError::Io { action, source, .. }) => {
            check!(eq; action, "creating the reached-boundary marker");
            check!(eq; source.kind(), std::io::ErrorKind::NotFound);
        }
        other => return Err(format!("expected missing marker directory, got {other:?}").into()),
    }
    check!(
        !marker.exists(),
        "failed publication must not signal a reached boundary"
    );
    std::fs::create_dir(&parent)?;

    match reached(&store, &operation, &consolidated) {
        Err(BoundaryError::HoldExpired {
            operation: held,
            millis,
        }) => {
            check!(eq; held, operation);
            check!(eq; millis, 1);
        }
        other => {
            return Err(format!("retry must reach its unconsumed boundary, got {other:?}").into())
        }
    }
    bind_evidence(&marker_value(&marker)?, &store, &tables)?;
    Ok(())
}
