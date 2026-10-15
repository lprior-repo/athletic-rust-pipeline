use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;

use crate::PublicIdentityError;

use super::support::{accepted, refused};
use super::*;

#[test]
fn public_identity_historical_v1_refuses_fresh_identity_write_without_mutation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("historical");
    Store::create_public_identity_qualification_v1(&root)?;
    let (store, writer) = Store::open_public_identity_qualification_v1(&root)?.into_parts();
    check!(eq; store.store_format()?.lifecycle_revision, Some(5));
    check!(eq; writer.qualification_version(), PublicIdentityQualificationVersion::SourceBoundV1);
    let athlete = subject("historical-write-refusal", "116")?;
    accepted(&athlete)?;
    store.append(Table::Athletes, &athlete)?;
    let head = writer.head(&store)?;
    check!(eq; head.sequence, 0);
    let before = retained_state(&store)?;
    let error = refused(writer.establish_source_bound(
        &store,
        "historical-fresh-write",
        &athlete.id.cast(),
        store.snapshot().revision(),
        &head,
    ))?;
    check!(matches!(
        error,
        StoreError::PublicIdentity(PublicIdentityError::QualificationVersionMismatch {
            expected: PublicIdentityQualificationVersion::RetainedAuthorityContextV2,
            found: PublicIdentityQualificationVersion::SourceBoundV1,
        })
    ));
    check!(eq; writer.head(&store)?, head);
    check!(eq; store.count(Table::AthleteIdentityDecisions)?, 0);
    check!(eq; retained_state(&store)?, before);
    drop(store);
    check!(eq; physical_state(&root)?, before);
    let (store, writer) = Store::open_public_identity_qualification_v1(&root)?.into_parts();
    check!(eq; writer.head(&store)?, head);
    check!(eq; writer.qualification_version(), PublicIdentityQualificationVersion::SourceBoundV1);
    check!(eq; retained_state(&store)?, before);
    Ok(())
}
