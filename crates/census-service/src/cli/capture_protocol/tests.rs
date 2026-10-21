use super::*;
use census_domain::model::{CaptureProtocolVersion, CaptureQualification};
use census_domain::public_identity_qualification::PublicIdentityQualificationV3;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn explicit_cli_init_binds_current_graph_and_capture_without_reinitialization() -> TestResult {
    let root = tempfile::tempdir()?;
    let path = root.path().join("fresh");
    let mut initialized = Vec::new();
    run_to(
        Some(&path),
        &CaptureProtocolArgs {
            command: CaptureProtocolCommand::Init,
        },
        &mut initialized,
    )?;
    let identity: PublicIdentityQualificationV3 = serde_json::from_slice(&initialized)?;
    let mut shown = Vec::new();
    run_to(
        Some(&path),
        &CaptureProtocolArgs {
            command: CaptureProtocolCommand::Show,
        },
        &mut shown,
    )?;
    let record = CaptureQualification::V2(serde_json::from_slice(&shown)?);
    check!(eq; record.store_identity(), identity.store_identity());
    check!(eq; record.protocol(), CaptureProtocolVersion::V2);
    let (store, writer) = Store::open_current_qualification_v3(&path)?.into_parts();
    check!(eq; writer.store_identity(), identity.store_identity());
    drop((store, writer));
    let (store, writer) = Store::open_public_identity_qualification_v3(&path)?.into_parts();
    check!(eq; writer.store_identity(), identity.store_identity());
    drop((store, writer));
    check!(Store::open(&path).is_err());
    let mut refused = Vec::new();
    let error = run_to(
        Some(&path),
        &CaptureProtocolArgs {
            command: CaptureProtocolCommand::Init,
        },
        &mut refused,
    )
    .err()
    .ok_or("current root was reinitialized")?;
    check!(matches!(
        error.downcast_ref::<census_store::StoreError>(),
        Some(census_store::StoreError::PublicIdentity(
            census_store::PublicIdentityError::ExistingRoot
        ))
    ));
    check!(refused.is_empty());
    check!(eq; Store::capture_qualification_control_existing(&path)?.show()?, record);
    Ok(())
}

#[test]
fn current_cli_does_not_upgrade_historical_capture_qualification() -> TestResult {
    let parent = tempfile::tempdir()?;
    let path = parent.path().join("historical-v1");
    let original = Store::create_historical_capture_v1(&path)?;
    let mut output = Vec::new();
    let error = run_to(
        Some(&path),
        &CaptureProtocolArgs {
            command: CaptureProtocolCommand::Init,
        },
        &mut output,
    )
    .err()
    .ok_or("v1 root was upgraded")?;
    check!(matches!(
        error.downcast_ref::<census_store::StoreError>(),
        Some(census_store::StoreError::PublicIdentity(
            census_store::PublicIdentityError::ExistingRoot
        ))
    ));
    check!(output.is_empty());
    check!(eq; Store::capture_qualification_control_existing(&path)?.show()?, CaptureQualification::V1(original));
    Ok(())
}
