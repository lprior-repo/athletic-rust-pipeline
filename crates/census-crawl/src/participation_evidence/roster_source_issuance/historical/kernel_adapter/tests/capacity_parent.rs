use super::super::capacity_parent::{
    MANIFEST_NATIVE_CONFIGURATION, MANIFEST_PORTABLE_CONFIGURATION,
};
use super::super::directory_parent::{
    ALTERNATE_ITERATION_CONFIGURATION, FAST_ITERATION_CONFIGURATION,
};
use super::super::execution::{RAW_VALUE_NATIVE_CONFIGURATION, RAW_VALUE_PORTABLE_CONFIGURATION};
use super::*;

const NATIVE: &[u8] = include_bytes!("../fixtures/kernel-v13-before-roster-capacity-native.json");
const PORTABLE: &[u8] =
    include_bytes!("../fixtures/kernel-v13-before-roster-capacity-portable.json");
const NATIVE_TEMPLATE: &[u8] =
    include_bytes!("../fixtures/kernel-v13-before-roster-capacity-native-template.json");
const FAST: &[u8] = include_bytes!("../fixtures/kernel-v13-before-roster-capacity-fast.json");

type OriginalDocument = (
    &'static [u8],
    &'static str,
    usize,
    &'static [u8; 32],
    &'static [[u8; 32]],
);

fn originals() -> [OriginalDocument; 4] {
    [
        (
            NATIVE,
            "86cad2fdab29ea9a175438fa0a8b8dc3b89db8aea627f5ff0eda83b30e0de791",
            40_155,
            &MANIFEST_NATIVE_CONFIGURATION,
            &[
                MANIFEST_NATIVE_CONFIGURATION,
                MANIFEST_PORTABLE_CONFIGURATION,
            ],
        ),
        (
            PORTABLE,
            "09a2757096e47bd8c3566a4d6c33395f4b79a528286fdeb292ce466489656066",
            40_031,
            &MANIFEST_PORTABLE_CONFIGURATION,
            &[MANIFEST_PORTABLE_CONFIGURATION],
        ),
        (
            NATIVE_TEMPLATE,
            "4b993ed2a67718f958ef84368541be2c0c3b9b590a3f4b611db90c162dce0c83",
            40_156,
            &RAW_VALUE_NATIVE_CONFIGURATION,
            &[
                RAW_VALUE_NATIVE_CONFIGURATION,
                RAW_VALUE_PORTABLE_CONFIGURATION,
                MANIFEST_PORTABLE_CONFIGURATION,
            ],
        ),
        (
            FAST,
            "d144164145a972d64aef7a79a9059099c69b28c098fe0c6cc20db7c284258d1c",
            40_156,
            &FAST_ITERATION_CONFIGURATION,
            &[FAST_ITERATION_CONFIGURATION],
        ),
    ]
}

#[test]
fn original_capacity_documents_keep_literal_bytes_and_actual_contexts() -> TestResult {
    let (current, current_digest) = executing(KernelEpoch::RowLocal);
    for (retained, digest, bytes, actual_context, _) in originals() {
        check!(format!("{:x}", Sha256::digest(retained)) == digest);
        check!(retained.len() == bytes);
        check!(&context::digest(retained)? == actual_context);
        check!(retained != current.as_bytes());
        check!(digest != current_digest);
        let document: serde_json::Value = serde_json::from_slice(retained)?;
        check!(
            document
                .get("source_digest")
                .and_then(serde_json::Value::as_str)
                == Some("2aa8f8d8b9a1c3b7df2e183dfb6618f8392e85e5057e3d8b5328b540bb8da450")
        );
        check!(
            document
                .get("sources")
                .and_then(serde_json::Value::as_array)
                .map(Vec::len)
                == Some(208)
        );
    }
    Ok(())
}

#[test]
fn original_capacity_documents_keep_exact_prior_references_and_v2_owner() -> TestResult {
    let (current, digest) = executing(KernelEpoch::RowLocal);
    for (retained, original_digest, bytes, _, contexts) in originals() {
        for allowed in contexts {
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
    }
    Ok(())
}

#[test]
fn original_capacity_documents_refuse_reverse_and_other_contexts_and_poison() -> TestResult {
    let contexts = [
        MANIFEST_NATIVE_CONFIGURATION,
        MANIFEST_PORTABLE_CONFIGURATION,
        RAW_VALUE_NATIVE_CONFIGURATION,
        RAW_VALUE_PORTABLE_CONFIGURATION,
        FAST_ITERATION_CONFIGURATION,
        ALTERNATE_ITERATION_CONFIGURATION,
        EXECUTING_CONFIGURATION,
        PORTABLE_EXECUTING_CONFIGURATION,
        ALTERNATE_CONFIGURATION,
        [0; 32],
    ];
    let (current, digest) = executing(KernelEpoch::RowLocal);
    for (retained, _, _, _, allowed) in originals() {
        for configuration in contexts.iter().filter(|held| !allowed.contains(*held)) {
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
            poisoned(&mut grant)?;
        }
    }
    Ok(())
}

#[test]
fn original_capacity_documents_refuse_the_full_epoch() -> TestResult {
    let (full, digest) = executing(KernelEpoch::Full);
    for (retained, _, _, actual, _) in originals() {
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            bind_historical_v1(
                KernelEpoch::Full,
                retained,
                full,
                digest,
                actual,
                &mut grant
            ),
            Err(StoreError::Refused { .. })
        ));
        poisoned(&mut grant)?;
    }
    Ok(())
}

#[test]
fn original_capacity_documents_refuse_appended_and_rehashed_drift() -> TestResult {
    let (current, digest) = executing(KernelEpoch::RowLocal);
    for (retained, _, _, actual, _) in originals() {
        let mut appended = retained.to_vec();
        appended.push(b'\n');
        let mut altered: serde_json::Value = serde_json::from_slice(retained)?;
        let sources = altered
            .get_mut("sources")
            .and_then(serde_json::Value::as_array_mut)
            .ok_or("kernel sources")?;
        let first = sources.first_mut().ok_or("first source")?;
        let sha = first.get_mut("sha256").ok_or("first source digest")?;
        *sha = serde_json::Value::String("1".repeat(64));
        let source_digest = format!("{:x}", Sha256::digest(serde_json::to_vec(sources)?));
        *altered.get_mut("source_digest").ok_or("source digest")? =
            serde_json::Value::String(source_digest);
        for changed in [appended, serde_json::to_vec(&altered)?] {
            let mut grant = AttachmentReplayGrantV1::new();
            check!(matches!(
                bind_historical_v1(
                    KernelEpoch::RowLocal,
                    &changed,
                    current,
                    digest,
                    actual,
                    &mut grant,
                ),
                Err(StoreError::Refused { .. })
            ));
            poisoned(&mut grant)?;
        }
    }
    Ok(())
}

#[test]
fn portable_capacity_allowance_cannot_unlock_other_originals_or_current_native() -> TestResult {
    let retained = include_bytes!("../fixtures/kernel-v13-pre-directory-native-template.json");
    let (current, digest) = executing(KernelEpoch::RowLocal);
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        bind_historical_v1(
            KernelEpoch::RowLocal,
            retained,
            current,
            digest,
            &MANIFEST_PORTABLE_CONFIGURATION,
            &mut grant,
        ),
        Err(StoreError::Refused { .. })
    ));
    poisoned(&mut grant)?;
    let native = include_bytes!(concat!(
        env!("OUT_DIR"),
        "/roster-semantic-kernel-v13-native.json"
    ));
    check!(super::super::current::native(
        KernelEpoch::RowLocal,
        native,
        &MANIFEST_PORTABLE_CONFIGURATION,
        native.len().try_into()?,
    )
    .is_none());
    Ok(())
}

fn poisoned(grant: &mut AttachmentReplayGrantV1) -> TestResult {
    check!(matches!(
        grant.ensure_available(),
        Err(census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned)
    ));
    let (current, digest) = executing(KernelEpoch::RowLocal);
    check!(matches!(
        bind_historical_v1(
            KernelEpoch::RowLocal, current.as_bytes(), current, digest,
            &MANIFEST_PORTABLE_CONFIGURATION, grant,
        ),
        Err(StoreError::EvidenceSubjectAttachment(
            EvidenceSubjectAttachmentStoreError::Domain(
                census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned
            )
        ))
    ));
    Ok(())
}
