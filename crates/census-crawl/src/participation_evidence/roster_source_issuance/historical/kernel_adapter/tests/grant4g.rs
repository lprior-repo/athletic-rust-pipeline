use super::*;

fn native_cases() -> [(KernelEpoch, &'static [u8]); 2] {
    [
        (
            KernelEpoch::Full,
            include_bytes!("../fixtures/kernel-v12-grant4g-native.json"),
        ),
        (
            KernelEpoch::RowLocal,
            include_bytes!("../fixtures/kernel-v13-grant4g-native.json"),
        ),
    ]
}

fn portable_cases() -> [(KernelEpoch, &'static [u8]); 2] {
    [
        (
            KernelEpoch::Full,
            include_bytes!("../fixtures/kernel-v12-grant4g-portable.json"),
        ),
        (
            KernelEpoch::RowLocal,
            include_bytes!("../fixtures/kernel-v13-grant4g-portable.json"),
        ),
    ]
}

#[test]
fn superseded_kernels_keep_their_original_reference_and_v2_owner() -> TestResult {
    for (epoch, retained) in native_cases() {
        for configuration in [EXECUTING_CONFIGURATION, PORTABLE_EXECUTING_CONFIGURATION] {
            require_replay(epoch, retained, &configuration)?;
        }
    }
    for (epoch, retained) in portable_cases() {
        require_replay(epoch, retained, &PORTABLE_EXECUTING_CONFIGURATION)?;
    }
    Ok(())
}

#[test]
fn unqualified_contexts_refuse_without_a_reverse_portability_allowance() -> TestResult {
    for (epoch, retained) in native_cases().into_iter().chain(portable_cases()) {
        for configuration in [ALTERNATE_CONFIGURATION, [0; 32], [0xff; 32]] {
            require_refusal(epoch, retained, &configuration)?;
        }
    }
    for (epoch, retained) in portable_cases() {
        require_refusal(epoch, retained, &EXECUTING_CONFIGURATION)?;
    }
    Ok(())
}

#[test]
fn changed_original_bytes_and_wrong_inventory_epoch_poison_replay() -> TestResult {
    for (epoch, retained) in native_cases().into_iter().chain(portable_cases()) {
        let wrong_epoch = match epoch {
            KernelEpoch::Full => KernelEpoch::RowLocal,
            KernelEpoch::RowLocal => KernelEpoch::Full,
        };
        require_refusal(wrong_epoch, retained, &PORTABLE_EXECUTING_CONFIGURATION)?;
        let mut changed = retained.to_vec();
        changed.push(b'\n');
        require_refusal(epoch, &changed, &PORTABLE_EXECUTING_CONFIGURATION)?;
    }
    Ok(())
}

fn require_replay(epoch: KernelEpoch, retained: &[u8], configuration: &[u8; 32]) -> TestResult {
    let (current, digest) = executing(epoch);
    let mut grant = AttachmentReplayGrantV1::new();
    let checked = bind_historical_v1(epoch, retained, current, digest, configuration, &mut grant)?;
    checked.require_reference(
        &format!("{:x}", Sha256::digest(retained)),
        retained.len().try_into()?,
    )?;
    checked.require_semantic_contract(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V2)?;
    check!(matches!(
        checked.require_semantic_contract(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1),
        Err(StoreError::Refused { .. })
    ));
    grant.ensure_available()?;
    Ok(())
}

fn require_refusal(epoch: KernelEpoch, retained: &[u8], configuration: &[u8; 32]) -> TestResult {
    let (current, digest) = executing(epoch);
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        bind_historical_v1(epoch, retained, current, digest, configuration, &mut grant),
        Err(StoreError::Refused { .. })
    ));
    check!(matches!(
        grant.charge_steps(0),
        Err(census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}
