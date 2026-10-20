use super::super::execution::RAW_VALUE_NATIVE_CONFIGURATION;
use super::*;

#[test]
fn raw_value_native_context_authenticates_original_predecessors_with_v1_semantics() -> TestResult {
    let cases: &[(KernelEpoch, &[u8], &'static str)] = &[
        (
            KernelEpoch::Full,
            include_bytes!("../fixtures/kernel-v8-native.json"),
            "8a4c9f7f31199c9f856ed43f9905ed01741cfa506707c8cd472cc6032b331b6f",
        ),
        (
            KernelEpoch::RowLocal,
            include_bytes!("../fixtures/kernel-v9-native.json"),
            "94ad0cb66c947e1fdc1efd31cbd99c7d1aac3bbc580bf1e81debfd16bf188505",
        ),
    ];
    for &(epoch, retained, expected_digest) in cases {
        let (current, digest) = executing(epoch);
        let mut grant = AttachmentReplayGrantV1::new();
        let checked = bind_historical_v1(
            epoch,
            retained,
            current,
            digest,
            &RAW_VALUE_NATIVE_CONFIGURATION,
            &mut grant,
        )?;
        checked.require_reference(expected_digest, retained.len().try_into()?)?;
        checked.require_semantic_contract(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1)?;
        check!(matches!(
            checked.require_semantic_contract(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V2),
            Err(StoreError::Refused { .. })
        ));
        grant.ensure_available()?;
    }
    Ok(())
}

#[test]
fn raw_value_native_context_one_bit_change_refuses_and_poisons() -> TestResult {
    let mut changed = RAW_VALUE_NATIVE_CONFIGURATION;
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
fn raw_value_native_context_refuses_alternate_documents() -> TestResult {
    let alternate_cases: &[(KernelEpoch, &[u8])] = &[
        (
            KernelEpoch::Full,
            include_bytes!("../fixtures/kernel-v8-alternate.json"),
        ),
        (
            KernelEpoch::RowLocal,
            include_bytes!("../fixtures/kernel-v9-alternate.json"),
        ),
    ];
    for &(epoch, retained) in alternate_cases {
        let (current, digest) = executing(epoch);
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            bind_historical_v1(
                epoch,
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
    }
    Ok(())
}

#[test]
fn raw_value_native_context_refuses_tampered_original_documents() -> TestResult {
    for (epoch, retained) in [
        (
            KernelEpoch::Full,
            include_bytes!("../fixtures/kernel-v8-native.json").as_slice(),
        ),
        (
            KernelEpoch::RowLocal,
            include_bytes!("../fixtures/kernel-v9-native.json").as_slice(),
        ),
    ] {
        let (current, digest) = executing(epoch);
        let mut tampered = retained.to_vec();
        tampered.push(b'\n');
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            bind_historical_v1(
                epoch,
                &tampered,
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
    }
    Ok(())
}
