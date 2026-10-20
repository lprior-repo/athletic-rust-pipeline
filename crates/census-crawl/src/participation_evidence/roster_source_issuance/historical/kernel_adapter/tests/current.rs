use super::super::current::native;
use super::super::execution::{RAW_VALUE_NATIVE_CONFIGURATION, RAW_VALUE_PORTABLE_CONFIGURATION};
use super::*;

const FULL_NATIVE: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/roster-semantic-kernel-v12-native.json"
));
const ROW_NATIVE: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/roster-semantic-kernel-v13-native.json"
));

fn cases() -> [(KernelEpoch, &'static [u8]); 2] {
    [
        (KernelEpoch::Full, FULL_NATIVE),
        (KernelEpoch::RowLocal, ROW_NATIVE),
    ]
}

#[test]
fn current_native_counterpart_preserves_exact_original_reference_and_v2_owner() -> TestResult {
    for (epoch, retained) in cases() {
        let bytes = retained.len().try_into()?;
        let bound = native(epoch, retained, &RAW_VALUE_PORTABLE_CONFIGURATION, bytes)
            .ok_or("authentic current native counterpart refused")?;
        bound.require_reference(&format!("{:x}", Sha256::digest(retained)), bytes)?;
        bound.require_semantic_contract(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V2)?;
        check!(matches!(
            bound.require_semantic_contract(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1),
            Err(StoreError::Refused { .. })
        ));
    }
    Ok(())
}

#[test]
fn changed_current_counterpart_bytes_refuse_and_poison_parent_replay() -> TestResult {
    for (epoch, retained) in cases() {
        let mut appended = retained.to_vec();
        appended.push(b'\n');
        let mut changed = retained.to_vec();
        *changed.first_mut().ok_or("native document header")? ^= 1;
        for tampered in [appended, changed] {
            let (executing, digest) = executing(epoch);
            let mut grant = AttachmentReplayGrantV1::new();
            check!(matches!(
                bind_historical_v1(
                    epoch,
                    &tampered,
                    executing,
                    digest,
                    &RAW_VALUE_PORTABLE_CONFIGURATION,
                    &mut grant,
                ),
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
fn wrong_epoch_cannot_use_current_native_counterpart() -> TestResult {
    for (epoch, retained) in [
        (KernelEpoch::Full, ROW_NATIVE),
        (KernelEpoch::RowLocal, FULL_NATIVE),
    ] {
        let (executing, digest) = executing(epoch);
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            bind_historical_v1(
                epoch,
                retained,
                executing,
                digest,
                &RAW_VALUE_PORTABLE_CONFIGURATION,
                &mut grant,
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
fn only_qualified_optimized_context_gets_the_cross_profile_counterpart() -> TestResult {
    let mut changed = RAW_VALUE_PORTABLE_CONFIGURATION;
    changed[0] ^= 1;
    let configurations = [
        EXECUTING_CONFIGURATION,
        PORTABLE_EXECUTING_CONFIGURATION,
        RAW_VALUE_NATIVE_CONFIGURATION,
        ALTERNATE_CONFIGURATION,
        changed,
        [0; 32],
    ];
    for (epoch, retained) in cases() {
        for configuration in &configurations {
            check!(native(epoch, retained, configuration, retained.len().try_into()?).is_none());
        }
    }
    Ok(())
}
