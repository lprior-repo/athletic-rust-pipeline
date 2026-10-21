use super::super::execution::{
    CACHE_TARGET_NATIVE_CONFIGURATION, RAW_VALUE_NATIVE_CONFIGURATION,
    RAW_VALUE_PORTABLE_CONFIGURATION, SHARED_FIXTURE_NATIVE_CONFIGURATION,
};
use super::*;

const FULL_PRE_GRADE_NATIVE: &[u8] = include_bytes!("../fixtures/kernel-v12-pre-grade-native.json");
const ROW_LOCAL_PRE_GRADE_NATIVE: &[u8] =
    include_bytes!("../fixtures/kernel-v13-pre-grade-native.json");
const FULL_PRE_GRADE_PORTABLE: &[u8] =
    include_bytes!("../fixtures/kernel-v12-pre-grade-portable.json");
const ROW_LOCAL_PRE_GRADE_PORTABLE: &[u8] =
    include_bytes!("../fixtures/kernel-v13-pre-grade-portable.json");

#[test]
fn shared_fixture_native_binds_original_pre_grade_native_as_v2_owner_with_unchanged_reference(
) -> TestResult {
    let cases = [
        (KernelEpoch::Full, FULL_PRE_GRADE_NATIVE),
        (KernelEpoch::RowLocal, ROW_LOCAL_PRE_GRADE_NATIVE),
    ];
    for (epoch, retained) in cases {
        let (current, digest) = executing(epoch);
        let mut grant = AttachmentReplayGrantV1::new();
        let bound = bind_historical_v1(
            epoch,
            retained,
            current,
            digest,
            &SHARED_FIXTURE_NATIVE_CONFIGURATION,
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
fn shared_fixture_native_binds_only_qualified_original_documents() -> TestResult {
    let cases = [
        (KernelEpoch::Full, FULL_PRE_GRADE_NATIVE),
        (KernelEpoch::RowLocal, ROW_LOCAL_PRE_GRADE_NATIVE),
    ];
    for (epoch, retained) in cases {
        let (current, digest) = executing(epoch);
        let configuration = context::digest(current.as_bytes())?;
        let independent_expected = configuration == RAW_VALUE_NATIVE_CONFIGURATION
            || configuration == RAW_VALUE_PORTABLE_CONFIGURATION
            || configuration == SHARED_FIXTURE_NATIVE_CONFIGURATION
            || configuration == CACHE_TARGET_NATIVE_CONFIGURATION;
        let mut grant = AttachmentReplayGrantV1::new();
        let result =
            bind_historical_v1(epoch, retained, current, digest, &configuration, &mut grant);
        if independent_expected {
            let bound = result?;
            bound.require_semantic_contract(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V2)?;
            grant.ensure_available()?;
        } else {
            check!(matches!(result, Err(StoreError::Refused { .. })));
            check!(matches!(
                grant.ensure_available(),
                Err(census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned)
            ));
        }
    }
    Ok(())
}

#[test]
fn shared_fixture_native_one_bit_change_refuses_and_poisons() -> TestResult {
    let mut changed = SHARED_FIXTURE_NATIVE_CONFIGURATION;
    changed[0] ^= 1;
    for (epoch, retained) in [
        (KernelEpoch::Full, FULL_PRE_GRADE_NATIVE),
        (KernelEpoch::RowLocal, ROW_LOCAL_PRE_GRADE_NATIVE),
    ] {
        let (current, digest) = executing(epoch);
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            bind_historical_v1(epoch, retained, current, digest, &changed, &mut grant),
            Err(StoreError::Refused { .. })
        ));
        check!(matches!(
            grant.ensure_available(),
            Err(census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned)
        ));
    }
    Ok(())
}

#[test]
fn shared_fixture_native_refuses_original_portable_pre_grade_documents() -> TestResult {
    let cases = [
        (KernelEpoch::Full, FULL_PRE_GRADE_PORTABLE),
        (KernelEpoch::RowLocal, ROW_LOCAL_PRE_GRADE_PORTABLE),
    ];
    for (epoch, retained) in cases {
        let (current, digest) = executing(epoch);
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            bind_historical_v1(
                epoch,
                retained,
                current,
                digest,
                &SHARED_FIXTURE_NATIVE_CONFIGURATION,
                &mut grant
            ),
            Err(StoreError::Refused { .. })
        ));
        check!(matches!(
            grant.ensure_available(),
            Err(census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned)
        ));
    }
    Ok(())
}

#[test]
fn shared_fixture_native_refuses_tampered_original_documents() -> TestResult {
    for (epoch, retained) in [
        (KernelEpoch::Full, FULL_PRE_GRADE_NATIVE.to_vec()),
        (KernelEpoch::RowLocal, ROW_LOCAL_PRE_GRADE_NATIVE.to_vec()),
    ] {
        let (current, digest) = executing(epoch);
        let mut tampered = retained;
        tampered.push(b'\n');
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            bind_historical_v1(
                epoch,
                &tampered,
                current,
                digest,
                &SHARED_FIXTURE_NATIVE_CONFIGURATION,
                &mut grant
            ),
            Err(StoreError::Refused { .. })
        ));
        check!(matches!(
            grant.ensure_available(),
            Err(census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned)
        ));
    }
    Ok(())
}

#[test]
fn unknown_zero_and_foreign_contexts_refuse_original_native_under_shared_fixture_policy(
) -> TestResult {
    let foreign = [0xff; 32];
    for (epoch, retained) in [
        (KernelEpoch::Full, FULL_PRE_GRADE_NATIVE),
        (KernelEpoch::RowLocal, ROW_LOCAL_PRE_GRADE_NATIVE),
    ] {
        let (current, digest) = executing(epoch);
        for configuration in [&[0; 32], &foreign] {
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
fn cache_target_context_refuses_portable_originals_and_one_bit_changes() -> TestResult {
    let mut changed = CACHE_TARGET_NATIVE_CONFIGURATION;
    changed[0] ^= 1;
    for (epoch, retained, executing_configuration) in [
        (
            KernelEpoch::Full,
            FULL_PRE_GRADE_PORTABLE,
            CACHE_TARGET_NATIVE_CONFIGURATION,
        ),
        (
            KernelEpoch::RowLocal,
            ROW_LOCAL_PRE_GRADE_PORTABLE,
            CACHE_TARGET_NATIVE_CONFIGURATION,
        ),
        (KernelEpoch::Full, FULL_PRE_GRADE_NATIVE, changed),
        (KernelEpoch::RowLocal, ROW_LOCAL_PRE_GRADE_NATIVE, changed),
    ] {
        let (current, digest) = executing(epoch);
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            bind_historical_v1(
                epoch,
                retained,
                current,
                digest,
                &executing_configuration,
                &mut grant
            ),
            Err(StoreError::Refused { .. })
        ));
        check!(matches!(
            grant.ensure_available(),
            Err(census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned)
        ));
    }
    Ok(())
}
