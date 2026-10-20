use super::super::execution::RAW_VALUE_NATIVE_CONFIGURATION;
use super::*;

const FULL_NATIVE: &[u8] = include_bytes!("../fixtures/kernel-v12-member-native.json");
const ROW_LOCAL_NATIVE: &[u8] = include_bytes!("../fixtures/kernel-v13-member-native.json");
const FULL_RAW_VALUE_NATIVE: &[u8] =
    include_bytes!("../fixtures/kernel-v12-member-raw-value-native.json");
const ROW_LOCAL_RAW_VALUE_NATIVE: &[u8] =
    include_bytes!("../fixtures/kernel-v13-member-raw-value-native.json");
const FULL_PORTABLE: &[u8] = include_bytes!("../fixtures/kernel-v12-member-portable.json");
const ROW_LOCAL_PORTABLE: &[u8] = include_bytes!("../fixtures/kernel-v13-member-portable.json");

#[test]
fn member_native_and_raw_value_native_keep_v2_in_qualified_contexts() -> TestResult {
    let cases = [
        (KernelEpoch::Full, FULL_NATIVE, &EXECUTING_CONFIGURATION),
        (
            KernelEpoch::RowLocal,
            ROW_LOCAL_NATIVE,
            &EXECUTING_CONFIGURATION,
        ),
        (
            KernelEpoch::Full,
            FULL_NATIVE,
            &PORTABLE_EXECUTING_CONFIGURATION,
        ),
        (
            KernelEpoch::RowLocal,
            ROW_LOCAL_NATIVE,
            &PORTABLE_EXECUTING_CONFIGURATION,
        ),
        (
            KernelEpoch::Full,
            FULL_NATIVE,
            &RAW_VALUE_NATIVE_CONFIGURATION,
        ),
        (
            KernelEpoch::RowLocal,
            ROW_LOCAL_NATIVE,
            &RAW_VALUE_NATIVE_CONFIGURATION,
        ),
        (
            KernelEpoch::Full,
            FULL_RAW_VALUE_NATIVE,
            &RAW_VALUE_NATIVE_CONFIGURATION,
        ),
        (
            KernelEpoch::RowLocal,
            ROW_LOCAL_RAW_VALUE_NATIVE,
            &RAW_VALUE_NATIVE_CONFIGURATION,
        ),
    ];
    for (epoch, retained, configuration) in cases {
        let (current, digest) = executing(epoch);
        let mut grant = AttachmentReplayGrantV1::new();
        let bound =
            bind_historical_v1(epoch, retained, current, digest, configuration, &mut grant)?;
        bound.require_reference(
            &format!("{:x}", Sha256::digest(retained)),
            retained.len().try_into()?,
        )?;
        bound.require_semantic_contract(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V2)?;
        check!(matches!(
            bound.require_semantic_contract(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1),
            Err(StoreError::Refused { .. })
        ));
        grant.ensure_available()?;
    }
    Ok(())
}

#[test]
fn member_portable_accepted_only_under_portable_context() -> TestResult {
    let cases = [
        (KernelEpoch::Full, FULL_PORTABLE),
        (KernelEpoch::RowLocal, ROW_LOCAL_PORTABLE),
    ];
    for (epoch, retained) in cases {
        let (current, digest) = executing(epoch);
        let mut grant = AttachmentReplayGrantV1::new();
        let bound = bind_historical_v1(
            epoch,
            retained,
            current,
            digest,
            &PORTABLE_EXECUTING_CONFIGURATION,
            &mut grant,
        )?;
        bound.require_reference(
            &format!("{:x}", Sha256::digest(retained)),
            retained.len().try_into()?,
        )?;
        bound.require_semantic_contract(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V2)?;
        grant.ensure_available()?;

        for configuration in [&EXECUTING_CONFIGURATION, &RAW_VALUE_NATIVE_CONFIGURATION] {
            let mut grant2 = AttachmentReplayGrantV1::new();
            check!(matches!(
                bind_historical_v1(epoch, retained, current, digest, configuration, &mut grant2),
                Err(StoreError::Refused { .. })
            ));
            check!(matches!(
            grant2.ensure_available(),
            Err(census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned)
        ));
        }
    }
    Ok(())
}

#[test]
fn member_raw_value_native_refuses_old_contexts() -> TestResult {
    let cases = [
        (KernelEpoch::Full, FULL_RAW_VALUE_NATIVE),
        (KernelEpoch::RowLocal, ROW_LOCAL_RAW_VALUE_NATIVE),
    ];
    for (epoch, retained) in cases {
        let (current, digest) = executing(epoch);
        for config in [&EXECUTING_CONFIGURATION, &PORTABLE_EXECUTING_CONFIGURATION] {
            let mut grant = AttachmentReplayGrantV1::new();
            check!(matches!(
                bind_historical_v1(epoch, retained, current, digest, config, &mut grant),
                Err(StoreError::Refused { .. })
            ));
            check!(matches!(
                grant.ensure_available(),
                Err(census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned)
            ));
        }
    }
    Ok(())
}

#[test]
fn member_wrong_epoch_and_tampered_refuse_and_poison() -> TestResult {
    let cases = [
        (KernelEpoch::Full, FULL_NATIVE, KernelEpoch::RowLocal),
        (KernelEpoch::RowLocal, ROW_LOCAL_NATIVE, KernelEpoch::Full),
    ];
    for (epoch, retained, wrong_epoch) in cases {
        let (current, digest) = executing(epoch);
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            bind_historical_v1(
                wrong_epoch,
                retained,
                current,
                digest,
                &RAW_VALUE_NATIVE_CONFIGURATION,
                &mut grant
            ),
            Err(StoreError::Refused { .. })
        ));
        check!(matches!(
            grant.ensure_available(),
            Err(census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned)
        ));

        let mut tampered = retained.to_vec();
        tampered.push(b'\n');
        let mut grant2 = AttachmentReplayGrantV1::new();
        check!(matches!(
            bind_historical_v1(
                epoch,
                &tampered,
                current,
                digest,
                &RAW_VALUE_NATIVE_CONFIGURATION,
                &mut grant2
            ),
            Err(StoreError::Refused { .. })
        ));
        check!(matches!(
            grant2.ensure_available(),
            Err(census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned)
        ));
    }
    Ok(())
}
