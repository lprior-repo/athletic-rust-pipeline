use super::super::execution::{RAW_VALUE_NATIVE_CONFIGURATION, RAW_VALUE_PORTABLE_CONFIGURATION};
use super::*;

#[test]
fn raw_value_portable_executing_reads_v8_v9_native_as_v1_owner() -> TestResult {
    let cases: &[(KernelEpoch, &[u8])] = &[
        (
            KernelEpoch::Full,
            include_bytes!("../fixtures/kernel-v8-native.json"),
        ),
        (
            KernelEpoch::RowLocal,
            include_bytes!("../fixtures/kernel-v9-native.json"),
        ),
    ];
    for &(epoch, retained) in cases {
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
        bound.require_semantic_contract(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1)?;
        check!(matches!(
            bound.require_semantic_contract(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V2),
            Err(StoreError::Refused { .. })
        ));
        grant.ensure_available()?;
    }
    Ok(())
}

#[test]
fn raw_value_portable_executing_reads_member_fixtures_as_v2_owner() -> TestResult {
    let cases: &[(KernelEpoch, &[u8])] = &[
        (
            KernelEpoch::Full,
            include_bytes!("../fixtures/kernel-v12-member-native.json"),
        ),
        (
            KernelEpoch::RowLocal,
            include_bytes!("../fixtures/kernel-v13-member-native.json"),
        ),
        (
            KernelEpoch::Full,
            include_bytes!("../fixtures/kernel-v12-member-raw-value-native.json"),
        ),
        (
            KernelEpoch::RowLocal,
            include_bytes!("../fixtures/kernel-v13-member-raw-value-native.json"),
        ),
        (
            KernelEpoch::Full,
            include_bytes!("../fixtures/kernel-v12-member-portable.json"),
        ),
        (
            KernelEpoch::RowLocal,
            include_bytes!("../fixtures/kernel-v13-member-portable.json"),
        ),
    ];
    for &(epoch, retained) in cases {
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
fn raw_value_portable_executing_reads_context_native_as_v2_owner() -> TestResult {
    let cases: &[(KernelEpoch, &[u8])] = &[
        (
            KernelEpoch::Full,
            include_bytes!("../fixtures/kernel-v12-context-native.json"),
        ),
        (
            KernelEpoch::RowLocal,
            include_bytes!("../fixtures/kernel-v13-context-native.json"),
        ),
    ];
    for &(epoch, retained) in cases {
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
        grant.ensure_available()?;
    }
    Ok(())
}

#[test]
fn raw_value_native_executing_reads_own_context_native_as_v2_owner() -> TestResult {
    let cases: &[(KernelEpoch, &[u8])] = &[
        (
            KernelEpoch::Full,
            include_bytes!("../fixtures/kernel-v12-context-native.json"),
        ),
        (
            KernelEpoch::RowLocal,
            include_bytes!("../fixtures/kernel-v13-context-native.json"),
        ),
    ];
    for &(epoch, retained) in cases {
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
        grant.ensure_available()?;
    }
    Ok(())
}

#[test]
fn raw_value_portable_one_bit_change_refuses_and_poisons() -> TestResult {
    let mut changed = RAW_VALUE_PORTABLE_CONFIGURATION;
    changed[0] ^= 1;
    for (epoch, retained) in [
        (
            KernelEpoch::Full,
            include_bytes!("../fixtures/kernel-v8-native.json").as_slice(),
        ),
        (
            KernelEpoch::RowLocal,
            include_bytes!("../fixtures/kernel-v9-native.json").as_slice(),
        ),
        (
            KernelEpoch::Full,
            include_bytes!("../fixtures/kernel-v12-member-portable.json").as_slice(),
        ),
        (
            KernelEpoch::RowLocal,
            include_bytes!("../fixtures/kernel-v13-context-native.json").as_slice(),
        ),
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
fn raw_value_portable_refuses_alternate_v8_v9_documents() -> TestResult {
    let cases: &[(KernelEpoch, &[u8])] = &[
        (
            KernelEpoch::Full,
            include_bytes!("../fixtures/kernel-v8-alternate.json"),
        ),
        (
            KernelEpoch::RowLocal,
            include_bytes!("../fixtures/kernel-v9-alternate.json"),
        ),
    ];
    for &(epoch, retained) in cases {
        let (current, digest) = executing(epoch);
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            bind_historical_v1(
                epoch,
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
    }
    Ok(())
}

#[test]
fn older_contexts_cannot_read_member_portable_fixtures() -> TestResult {
    for (epoch, retained) in [
        (
            KernelEpoch::Full,
            include_bytes!("../fixtures/kernel-v12-member-portable.json").as_slice(),
        ),
        (
            KernelEpoch::RowLocal,
            include_bytes!("../fixtures/kernel-v13-member-portable.json").as_slice(),
        ),
    ] {
        let (current, digest) = executing(epoch);
        for config in [&EXECUTING_CONFIGURATION, &RAW_VALUE_NATIVE_CONFIGURATION] {
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
fn older_contexts_cannot_read_context_native_fixtures() -> TestResult {
    for (epoch, retained) in [
        (
            KernelEpoch::Full,
            include_bytes!("../fixtures/kernel-v12-context-native.json").as_slice(),
        ),
        (
            KernelEpoch::RowLocal,
            include_bytes!("../fixtures/kernel-v13-context-native.json").as_slice(),
        ),
    ] {
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
