use super::*;

#[test]
fn exact_portable_context_preserves_each_supported_predecessor_contract() -> TestResult {
    let cases: &[(KernelEpoch, &[u8], Option<&str>)] = &[
        (
            KernelEpoch::Full,
            include_bytes!("../fixtures/kernel-v8-native.json"),
            Some(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1),
        ),
        (
            KernelEpoch::Full,
            include_bytes!("../fixtures/kernel-v8-alternate.json"),
            None,
        ),
        (
            KernelEpoch::RowLocal,
            include_bytes!("../fixtures/kernel-v9-native.json"),
            Some(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1),
        ),
        (
            KernelEpoch::RowLocal,
            include_bytes!("../fixtures/kernel-v9-alternate.json"),
            None,
        ),
        (
            KernelEpoch::Full,
            include_bytes!("../fixtures/kernel-v10-af9-native.json"),
            Some(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V2),
        ),
        (
            KernelEpoch::RowLocal,
            include_bytes!("../fixtures/kernel-v11-af9-native.json"),
            Some(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V2),
        ),
    ];
    for &(epoch, retained, expected) in cases {
        let (current, digest) = executing(epoch);
        let mut grant = AttachmentReplayGrantV1::new();
        let bound = bind_historical_v1(
            epoch,
            retained,
            current,
            digest,
            &PORTABLE_EXECUTING_CONFIGURATION,
            &mut grant,
        );
        match expected {
            Some(contract) => {
                let bound = bound?;
                bound.require_reference(
                    &format!("{:x}", Sha256::digest(retained)),
                    retained.len().try_into()?,
                )?;
                bound.require_semantic_contract(contract)?;
                let other = if contract == PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1 {
                    PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V2
                } else {
                    PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1
                };
                check!(matches!(
                    bound.require_semantic_contract(other),
                    Err(StoreError::Refused { .. })
                ));
                grant.ensure_available()?;
            }
            None => {
                check!(matches!(bound, Err(StoreError::Refused { .. })));
                check!(matches!(grant.ensure_available(), Err(census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned)));
            }
        }
    }
    Ok(())
}

#[test]
fn changed_portable_and_unknown_contexts_refuse_without_resetting_original_grant() -> TestResult {
    let mut changed = PORTABLE_EXECUTING_CONFIGURATION;
    changed[0] ^= 1;
    for configuration in [changed, [0xff; 32]] {
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
                include_bytes!("../fixtures/kernel-v10-af9-native.json").as_slice(),
            ),
            (
                KernelEpoch::RowLocal,
                include_bytes!("../fixtures/kernel-v11-af9-native.json").as_slice(),
            ),
        ] {
            let (current, digest) = executing(epoch);
            let mut grant = AttachmentReplayGrantV1::new();
            grant.charge_steps(7)?;
            check!(matches!(
                bind_historical_v1(epoch, retained, current, digest, &configuration, &mut grant),
                Err(StoreError::Refused { .. })
            ));
            check!(matches!(grant.ensure_available(), Err(census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned)));
        }
    }
    Ok(())
}
