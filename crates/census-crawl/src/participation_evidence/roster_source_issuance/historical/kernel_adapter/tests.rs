use super::*;
use std::sync::LazyLock;

mod af9;
mod grant4g;
mod member;
mod portable;
mod raw_value;
mod raw_value_portable;

type TestResult<E = Box<dyn std::error::Error>> = Result<(), E>;

const CURRENT_FULL: &str =
    include_str!(concat!(env!("OUT_DIR"), "/roster-semantic-kernel-v12.json"));
const CURRENT_ROWS: &str =
    include_str!(concat!(env!("OUT_DIR"), "/roster-semantic-kernel-v13.json"));
static FULL_DIGEST: LazyLock<String> =
    LazyLock::new(|| format!("{:x}", Sha256::digest(CURRENT_FULL.as_bytes())));
static ROW_DIGEST: LazyLock<String> =
    LazyLock::new(|| format!("{:x}", Sha256::digest(CURRENT_ROWS.as_bytes())));

fn executing(epoch: KernelEpoch) -> (&'static str, &'static str) {
    match epoch {
        KernelEpoch::Full => (CURRENT_FULL, FULL_DIGEST.as_str()),
        KernelEpoch::RowLocal => (CURRENT_ROWS, ROW_DIGEST.as_str()),
    }
}

#[test]
fn authentic_predecessors_bind_original_owner_policy_only_in_reviewed_execution_context(
) -> TestResult {
    let cases: &[(KernelEpoch, &[u8], &[u8; 32])] = &[
        (
            KernelEpoch::Full,
            include_bytes!("fixtures/kernel-v8-native.json"),
            &EXECUTING_CONFIGURATION,
        ),
        (
            KernelEpoch::Full,
            include_bytes!("fixtures/kernel-v8-alternate.json"),
            &ALTERNATE_CONFIGURATION,
        ),
        (
            KernelEpoch::RowLocal,
            include_bytes!("fixtures/kernel-v9-native.json"),
            &EXECUTING_CONFIGURATION,
        ),
        (
            KernelEpoch::RowLocal,
            include_bytes!("fixtures/kernel-v9-alternate.json"),
            &ALTERNATE_CONFIGURATION,
        ),
    ];
    for &(epoch, retained, allowed_context) in cases {
        let (current, digest) = executing(epoch);
        let configuration = context::digest(current.as_bytes())?;
        let mut grant = AttachmentReplayGrantV1::new();
        let bound =
            bind_historical_v1(epoch, retained, current, digest, &configuration, &mut grant);
        let reviewed_native = [
            EXECUTING_CONFIGURATION,
            PORTABLE_EXECUTING_CONFIGURATION,
            execution::RAW_VALUE_NATIVE_CONFIGURATION,
            execution::RAW_VALUE_PORTABLE_CONFIGURATION,
        ];
        let accepted = if allowed_context == &EXECUTING_CONFIGURATION {
            reviewed_native.contains(&configuration)
        } else {
            &configuration == allowed_context
        };
        if accepted {
            let bound = bound?;
            bound.require_reference(
                &format!("{:x}", Sha256::digest(retained)),
                retained.len().try_into()?,
            )?;
            bound.require_semantic_contract(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1)?;
            check!(matches!(
                bound.require_semantic_contract(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V2),
                Err(StoreError::Refused { .. })
            ));
        } else {
            check!(matches!(bound, Err(StoreError::Refused { .. })));
            check!(matches!(
                grant.ensure_available(),
                Err(census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned)
            ));
        }
    }
    Ok(())
}

#[test]
fn current_kernels_cannot_be_downgraded_to_the_historical_owner_policy() -> TestResult {
    for epoch in [KernelEpoch::Full, KernelEpoch::RowLocal] {
        let (current, digest) = executing(epoch);
        let configuration = context::digest(current.as_bytes())?;
        let mut grant = AttachmentReplayGrantV1::new();
        let bound = bind_historical_v1(
            epoch,
            current.as_bytes(),
            current,
            digest,
            &configuration,
            &mut grant,
        )?;
        bound.require_reference(digest, current.len().try_into()?)?;
        bound.require_semantic_contract(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V2)?;
        check!(matches!(
            bound.require_semantic_contract(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1),
            Err(StoreError::Refused { .. })
        ));
    }
    Ok(())
}

#[test]
fn changed_predecessor_bytes_poison_replay_and_cannot_fall_back_to_current() -> TestResult {
    for (epoch, retained) in [
        (
            KernelEpoch::Full,
            include_bytes!("fixtures/kernel-v8-native.json").as_slice(),
        ),
        (
            KernelEpoch::RowLocal,
            include_bytes!("fixtures/kernel-v9-native.json").as_slice(),
        ),
    ] {
        let (current, digest) = executing(epoch);
        let configuration = context::digest(current.as_bytes())?;
        let mut tampered = retained.to_vec();
        tampered.push(b'\n');
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            bind_historical_v1(
                epoch,
                &tampered,
                current,
                digest,
                &configuration,
                &mut grant
            ),
            Err(StoreError::Refused { .. })
        ));
        check!(matches!(
            grant.ensure_available(),
            Err(census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned)
        ));
        check!(matches!(
            bind_historical_v1(epoch, current.as_bytes(), current, digest, &configuration, &mut grant),
            Err(StoreError::EvidenceSubjectAttachment(
                EvidenceSubjectAttachmentStoreError::Domain(
                    census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned
                )
            ))
        ));
    }
    Ok(())
}

#[test]
fn known_original_documents_refuse_an_unqualified_executing_context() -> TestResult {
    for (epoch, retained) in [
        (
            KernelEpoch::Full,
            include_bytes!("fixtures/kernel-v8-native.json").as_slice(),
        ),
        (
            KernelEpoch::RowLocal,
            include_bytes!("fixtures/kernel-v9-native.json").as_slice(),
        ),
    ] {
        let (current, digest) = executing(epoch);
        let mut configuration = context::digest(current.as_bytes())?;
        configuration[0] ^= 1;
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            bind_historical_v1(epoch, retained, current, digest, &configuration, &mut grant),
            Err(StoreError::Refused { .. })
        ));
        check!(matches!(
            grant.ensure_available(),
            Err(census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned)
        ));
    }
    Ok(())
}
