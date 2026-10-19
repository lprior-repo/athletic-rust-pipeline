use super::*;

const AF9_FULL: &[u8] = include_bytes!("../fixtures/kernel-v10-af9-native.json");
const AF9_ROWS: &[u8] = include_bytes!("../fixtures/kernel-v11-af9-native.json");
const UNQUALIFIED_AF9_CONFIGURATION: [u8; 32] = [
    0xf7, 0x4e, 0x8e, 0x56, 0x07, 0x2f, 0x53, 0xec, 0xaf, 0xba, 0x06, 0xef, 0x4a, 0x8b, 0xa2, 0x72,
    0x9b, 0x26, 0x9d, 0xf9, 0xbd, 0x23, 0xf6, 0x3b, 0xe0, 0xa0, 0x61, 0x40, 0xb3, 0xf2, 0x12, 0x74,
];

fn cases() -> [(KernelEpoch, &'static [u8], &'static str); 2] {
    [
        (
            KernelEpoch::Full,
            AF9_FULL,
            "35695fab34f7e130fe3c3560e0568edf0897b837488ee7761864103176d6dd1b",
        ),
        (
            KernelEpoch::RowLocal,
            AF9_ROWS,
            "7c2fb5be2b9948e8df8c829fd9ac09ac7524d693ca980b3df1c81f5908ad858b",
        ),
    ]
}

#[test]
fn authentic_af9_native_documents_bind_v2_without_consuming_the_grant() -> TestResult {
    for (epoch, retained, expected_digest) in cases() {
        let (current, digest) = executing(epoch);
        let mut grant = AttachmentReplayGrantV1::new();
        let checked = bind_historical_v1(
            epoch,
            retained,
            current,
            digest,
            &EXECUTING_CONFIGURATION,
            &mut grant,
        )?;
        checked.require_reference(expected_digest, 21_507)?;
        check!(eq; checked.semantic_contract(), PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V2);
        checked.require_semantic_contract(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V2)?;
        check!(matches!(
            checked.require_semantic_contract(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1),
            Err(StoreError::Refused { .. })
        ));
        grant.ensure_available()?;
        grant.charge_steps(1)?;
    }
    Ok(())
}

#[test]
fn authentic_af9_documents_obey_the_real_compiled_context_fence() -> TestResult {
    for (epoch, retained, expected_digest) in cases() {
        let (current, digest) = executing(epoch);
        let configuration = context::digest(current.as_bytes())?;
        let mut grant = AttachmentReplayGrantV1::new();
        let result =
            bind_historical_v1(epoch, retained, current, digest, &configuration, &mut grant);
        if configuration == EXECUTING_CONFIGURATION {
            let checked = result?;
            checked.require_reference(expected_digest, 21_507)?;
            checked.require_semantic_contract(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V2)?;
            grant.ensure_available()?;
        } else {
            require_refusal(result, &grant)?;
        }
    }
    Ok(())
}

#[test]
fn af9_native_documents_refuse_other_contexts_and_poison_the_original_grant() -> TestResult {
    for (epoch, retained, _) in cases() {
        let (current, digest) = executing(epoch);
        for configuration in [
            ALTERNATE_CONFIGURATION,
            UNQUALIFIED_AF9_CONFIGURATION,
            [0; 32],
        ] {
            let mut grant = AttachmentReplayGrantV1::new();
            let result =
                bind_historical_v1(epoch, retained, current, digest, &configuration, &mut grant);
            require_refusal(result, &grant)?;
            check!(matches!(
                bind_historical_v1(
                    epoch, current.as_bytes(), current, digest,
                    &EXECUTING_CONFIGURATION, &mut grant,
                ),
                Err(StoreError::EvidenceSubjectAttachment(
                    EvidenceSubjectAttachmentStoreError::Domain(
                        census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned
                    )
                ))
            ));
        }
    }
    Ok(())
}

#[test]
fn af9_identity_requires_the_exact_document_and_inventory_flavor() -> TestResult {
    for (epoch, retained, _) in cases() {
        let (current, digest) = executing(epoch);
        let mut changed = retained.to_vec();
        changed.push(b'\n');
        let swapped = match epoch {
            KernelEpoch::Full => AF9_ROWS,
            KernelEpoch::RowLocal => AF9_FULL,
        };
        for bytes in [changed.as_slice(), swapped] {
            let mut grant = AttachmentReplayGrantV1::new();
            let result = bind_historical_v1(
                epoch,
                bytes,
                current,
                digest,
                &EXECUTING_CONFIGURATION,
                &mut grant,
            );
            require_refusal(result, &grant)?;
        }
    }
    Ok(())
}

fn require_refusal(
    result: StoreResult<CheckedKernelReplayV1>,
    grant: &AttachmentReplayGrantV1,
) -> TestResult {
    check!(matches!(result, Err(StoreError::Refused { .. })));
    check!(matches!(
        grant.ensure_available(),
        Err(census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}
