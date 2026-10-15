use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;

use crate::PublicIdentityError;

use super::support::refused;
use super::*;

#[test]
fn public_identity_historical_v1_refuses_every_mutating_writer_without_mutation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("historical");
    let declaration = crate::public_identity::create_historical_test_fixture(
        &root,
        PublicIdentityQualificationVersion::SourceBoundV1,
    )?;
    let before = physical_state(&root)?;
    let accepted = Store::replay_historical_public_identity(&root)?;
    check!(eq; accepted.qualification_version(), PublicIdentityQualificationVersion::SourceBoundV1);
    check!(eq; accepted.store_identity(), declaration.store_identity());
    check!(eq; accepted.head(), &census_domain::public_identity_history::genesis(declaration.store_identity())?);
    check!(accepted.history_records().is_empty());
    check!(eq; accepted.public_ids().count(), 0);
    check!(eq; physical_state(&root)?, before);
    for (result, found) in [
        (
            Store::open_public_identity_qualification_v1(&root),
            PublicIdentityQualificationVersion::SourceBoundV1,
        ),
        (
            Store::open_public_identity_qualification_v2(&root),
            PublicIdentityQualificationVersion::RetainedAuthorityContextV2,
        ),
        (
            Store::open_public_identity_qualification_v3(&root),
            PublicIdentityQualificationVersion::SourceBoundV1,
        ),
    ] {
        check!(matches!(refused(result)?, StoreError::PublicIdentity(
            PublicIdentityError::QualificationVersionMismatch {
                expected: PublicIdentityQualificationVersion::RetainedAuthorityGraphV3,
                found: actual,
            }
        ) if actual == found));
        check!(eq; physical_state(&root)?, before);
    }
    let reopened = Store::replay_historical_public_identity(&root)?;
    check!(eq; reopened.qualification_version(), accepted.qualification_version());
    check!(eq; reopened.store_identity(), accepted.store_identity());
    check!(eq; reopened.head(), accepted.head());
    check!(eq; physical_state(&root)?, before);
    Ok(())
}
