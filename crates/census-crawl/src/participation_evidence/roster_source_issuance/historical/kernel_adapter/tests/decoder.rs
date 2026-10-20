use super::super::execution::{RAW_VALUE_NATIVE_CONFIGURATION, RAW_VALUE_PORTABLE_CONFIGURATION};
use super::*;

const FULL_DECODER_NATIVE: &[u8] = include_bytes!("../fixtures/kernel-v12-decoder-native.json");
const ROW_LOCAL_DECODER_NATIVE: &[u8] =
    include_bytes!("../fixtures/kernel-v13-decoder-native.json");
const FULL_DECODER_PORTABLE: &[u8] = include_bytes!("../fixtures/kernel-v12-decoder-portable.json");
const ROW_LOCAL_DECODER_PORTABLE: &[u8] =
    include_bytes!("../fixtures/kernel-v13-decoder-portable.json");

#[test]
fn decoder_originals_bind_under_portable_raw_value_with_v2_owner() -> TestResult {
    let cases = [
        (KernelEpoch::Full, FULL_DECODER_NATIVE),
        (KernelEpoch::RowLocal, ROW_LOCAL_DECODER_NATIVE),
        (KernelEpoch::Full, FULL_DECODER_PORTABLE),
        (KernelEpoch::RowLocal, ROW_LOCAL_DECODER_PORTABLE),
    ];
    for (epoch, retained) in cases {
        let (current, digest) = executing(epoch);
        let mut grant = AttachmentReplayGrantV1::new();
        let bound = bind_historical_v1(
            epoch,
            retained,
            current,
            digest,
            &RAW_VALUE_PORTABLE_CONFIGURATION,
            &mut grant,
        )?;
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
fn decoder_native_also_binds_under_native_raw_value_with_v2_owner() -> TestResult {
    let cases = [
        (KernelEpoch::Full, FULL_DECODER_NATIVE),
        (KernelEpoch::RowLocal, ROW_LOCAL_DECODER_NATIVE),
    ];
    for (epoch, retained) in cases {
        let (current, digest) = executing(epoch);
        let mut grant = AttachmentReplayGrantV1::new();
        let bound = bind_historical_v1(
            epoch,
            retained,
            current,
            digest,
            &RAW_VALUE_NATIVE_CONFIGURATION,
            &mut grant,
        )?;
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
fn decoder_portable_refuses_native_and_legacy_contexts() -> TestResult {
    let cases = [
        (KernelEpoch::Full, FULL_DECODER_PORTABLE),
        (KernelEpoch::RowLocal, ROW_LOCAL_DECODER_PORTABLE),
    ];
    for (epoch, retained) in cases {
        let (current, digest) = executing(epoch);
        for configuration in [
            &EXECUTING_CONFIGURATION,
            &PORTABLE_EXECUTING_CONFIGURATION,
            &RAW_VALUE_NATIVE_CONFIGURATION,
        ] {
            let mut grant = AttachmentReplayGrantV1::new();
            check!(matches!(
                bind_historical_v1(epoch, retained, current, digest, configuration, &mut grant),
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
fn decoder_wrong_epoch_and_tampered_refuse_and_poison() -> TestResult {
    let cases = [
        (
            KernelEpoch::Full,
            FULL_DECODER_NATIVE,
            KernelEpoch::RowLocal,
        ),
        (
            KernelEpoch::RowLocal,
            ROW_LOCAL_DECODER_NATIVE,
            KernelEpoch::Full,
        ),
        (
            KernelEpoch::Full,
            FULL_DECODER_PORTABLE,
            KernelEpoch::RowLocal,
        ),
        (
            KernelEpoch::RowLocal,
            ROW_LOCAL_DECODER_PORTABLE,
            KernelEpoch::Full,
        ),
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
                &RAW_VALUE_PORTABLE_CONFIGURATION,
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
                &RAW_VALUE_PORTABLE_CONFIGURATION,
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
