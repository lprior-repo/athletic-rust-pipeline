use super::super::directory_parent::{
    ALTERNATE_ITERATION_CONFIGURATION, FAST_ITERATION_CONFIGURATION,
};
use super::super::execution::{
    CACHE_TARGET_NATIVE_CONFIGURATION, RAW_VALUE_NATIVE_CONFIGURATION,
    RAW_VALUE_PORTABLE_CONFIGURATION, SHARED_FIXTURE_NATIVE_CONFIGURATION,
};
use super::*;

const CURRENT: &[u8] = include_bytes!("../fixtures/kernel-v13-pre-directory-current.json");
const ALTERNATE: &[u8] = include_bytes!("../fixtures/kernel-v13-pre-directory-alternate.json");
const NATIVE_TEMPLATE: &[u8] =
    include_bytes!("../fixtures/kernel-v13-pre-directory-native-template.json");

type OriginalDocument = (
    &'static [u8],
    &'static str,
    usize,
    &'static [u8; 32],
    &'static [u8; 32],
);

fn originals() -> [OriginalDocument; 3] {
    [
        (
            CURRENT,
            "8ee87e8ad1dbf6ec23a90d535ff903666d258573485dac907ee496fd81be3968",
            21_556,
            &FAST_ITERATION_CONFIGURATION,
            &FAST_ITERATION_CONFIGURATION,
        ),
        (
            ALTERNATE,
            "e67e400e34985e703f1ddfa77880d71ddcc73c0060bd0b6718f45d00cb0d1052",
            21_677,
            &ALTERNATE_ITERATION_CONFIGURATION,
            &ALTERNATE_ITERATION_CONFIGURATION,
        ),
        (
            NATIVE_TEMPLATE,
            "6c1bc1b79f18b5dedba649b600510fcdb092336e8f7630ef302d85616b941ce1",
            21_556,
            &RAW_VALUE_NATIVE_CONFIGURATION,
            &RAW_VALUE_PORTABLE_CONFIGURATION,
        ),
    ]
}

#[test]
fn original_pre_directory_documents_keep_literal_bytes_and_actual_contexts() -> TestResult {
    let (current, current_digest) = executing(KernelEpoch::RowLocal);
    for (retained, digest, bytes, actual_context, _) in originals() {
        check!(format!("{:x}", Sha256::digest(retained)) == digest);
        check!(retained.len() == bytes);
        check!(&context::digest(retained)? == actual_context);
        check!(retained != current.as_bytes());
        check!(digest != current_digest);
        let document: serde_json::Value = serde_json::from_slice(retained)?;
        check!(
            document.get("schema").and_then(serde_json::Value::as_str)
                == Some("roster-semantic-kernel-dependencies/v13")
        );
        check!(
            document
                .get("source_digest")
                .and_then(serde_json::Value::as_str)
                == Some("51a3431587aa1d8cb48e2b0c29e0b936aebf1125c4818f72ddb48dce1c356aa9")
        );
        check!(
            document
                .get("sources")
                .and_then(serde_json::Value::as_array)
                .map(Vec::len)
                == Some(96)
        );
    }
    Ok(())
}

#[test]
fn original_pre_directory_documents_keep_exact_prior_v2_execution_allowances() -> TestResult {
    let (current, digest) = executing(KernelEpoch::RowLocal);
    for (retained, original_digest, bytes, _, allowed) in originals() {
        let mut grant = AttachmentReplayGrantV1::new();
        let bound = bind_historical_v1(
            KernelEpoch::RowLocal,
            retained,
            current,
            digest,
            allowed,
            &mut grant,
        )?;
        bound.require_reference(original_digest, bytes.try_into()?)?;
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
fn original_pre_directory_documents_refuse_all_other_known_contexts_and_poison() -> TestResult {
    let contexts = [
        FAST_ITERATION_CONFIGURATION,
        ALTERNATE_ITERATION_CONFIGURATION,
        RAW_VALUE_NATIVE_CONFIGURATION,
        RAW_VALUE_PORTABLE_CONFIGURATION,
        EXECUTING_CONFIGURATION,
        PORTABLE_EXECUTING_CONFIGURATION,
        ALTERNATE_CONFIGURATION,
        SHARED_FIXTURE_NATIVE_CONFIGURATION,
        CACHE_TARGET_NATIVE_CONFIGURATION,
        [0; 32],
    ];
    let (current, digest) = executing(KernelEpoch::RowLocal);
    for (retained, _, _, _, allowed) in originals() {
        for configuration in contexts
            .iter()
            .filter(|configuration| *configuration != allowed)
        {
            let mut grant = AttachmentReplayGrantV1::new();
            check!(matches!(
                bind_historical_v1(
                    KernelEpoch::RowLocal,
                    retained,
                    current,
                    digest,
                    configuration,
                    &mut grant,
                ),
                Err(StoreError::Refused { .. })
            ));
            poisoned(&grant)?;
            check!(matches!(bind_historical_v1(
                KernelEpoch::RowLocal, current.as_bytes(), current, digest, configuration, &mut grant,
            ), Err(StoreError::EvidenceSubjectAttachment(
                EvidenceSubjectAttachmentStoreError::Domain(
                    census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned
                )
            ))));
        }
    }
    Ok(())
}

#[test]
fn original_pre_directory_documents_refuse_other_epoch_and_changed_bytes() -> TestResult {
    let (current, digest) = executing(KernelEpoch::RowLocal);
    let (full, full_digest) = executing(KernelEpoch::Full);
    for (retained, _, _, _, allowed) in originals() {
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            bind_historical_v1(
                KernelEpoch::Full,
                retained,
                full,
                full_digest,
                allowed,
                &mut grant,
            ),
            Err(StoreError::Refused { .. })
        ));
        poisoned(&grant)?;
        let mut appended = retained.to_vec();
        appended.push(b'\n');
        let mut altered = retained.to_vec();
        let first = altered
            .first_mut()
            .ok_or("original kernel document is empty")?;
        *first ^= 1;
        for changed in [&appended, &altered] {
            let mut grant = AttachmentReplayGrantV1::new();
            check!(matches!(
                bind_historical_v1(
                    KernelEpoch::RowLocal,
                    changed,
                    current,
                    digest,
                    allowed,
                    &mut grant,
                ),
                Err(StoreError::Refused { .. })
            ));
            poisoned(&grant)?;
        }
    }
    Ok(())
}

fn poisoned(grant: &AttachmentReplayGrantV1) -> TestResult {
    check!(matches!(
        grant.ensure_available(),
        Err(census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}
