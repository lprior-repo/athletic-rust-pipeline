mod schema_three;

use super::super::codec::sha256_hex;
use super::super::stream::StreamKind;
use super::super::{cancel_freeze_v2, generation, INPUT_TABLES};
use super::*;
use crate::export::ExportDataset;

const PRIOR_POLICIES: [u32; 13] = [8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20];
const PRIOR_STATES: [Option<InputState>; 5] = [
    None,
    Some(InputState::Requested),
    Some(InputState::Freezing),
    Some(InputState::Frozen),
    Some(InputState::Abandoned),
];

fn prior_manifest(current: &FrozenManifest, policy: u32) -> TestResult<FrozenManifest> {
    let mut prior = current.clone();
    prior.lineage.policy_revision = policy;
    if matches!(policy, 14..=20) {
        prior.lineage.schema_revision = 3;
        prior.codec = "census-frozen-jsonl-v3".into();
        prior.codec_revision = 3;
        prior.input_token.protocol = 3;
        prior.input_token.logical_schema = 3;
    }
    prior.lineage.input_generation = generation(&prior.lineage)?;
    prior.input_token.semantic_policy = policy;
    prior.input_token.validate_against(&prior.lineage)?;
    if matches!(policy, 15..=20) {
        check!(eq; prior.streams.len(), 15);
        check!(eq; prior.source_fingerprints.len(), 16);
        for kind in [
            StreamKind::CoverageRows,
            StreamKind::CoachLanes,
            StreamKind::Relays,
        ] {
            check!(eq; prior.stream(kind).ok_or("retained stream missing")?.rows, 1);
        }
    }
    check!(ne; prior.lineage.input_generation, current.lineage.input_generation);
    check!(ne; prior.input_token.digest()?, current.input_token.digest()?);
    Ok(prior)
}

fn install_prior_policy(
    store: &Store,
    directory: &Path,
    manifest: &FrozenManifest,
    state: Option<InputState>,
) -> TestResult<Retained> {
    let encoded = serialized_bytes(manifest)?;
    if matches!(manifest.lineage.policy_revision, 19..=20) {
        check!(eq; FrozenManifest::decode(&encoded)?, *manifest);
    } else {
        let expected = format!("policy revision {}", manifest.lineage.policy_revision);
        check!(
            matches!(
                FrozenManifest::decode(&encoded),
                Err(FrozenCodecError::UnsupportedVersion { what: "manifest lineage", ref found, .. })
                    if found.contains(&expected)
            ) || matches!(
                FrozenManifest::decode(&encoded),
                Err(FrozenCodecError::UnsupportedVersion { what: "frozen manifest", ref found, .. })
                    if found == "census-frozen-jsonl-v2 revision 2"
            )
        );
    }
    let manifest_digest = objects::install(&objects::directory(directory), &encoded)?;
    check!(eq; objects::read(&objects::directory(directory), &manifest_digest, objects::MANIFEST_CEILING)?, encoded);
    let inventory = if manifest.codec_revision == 2 {
        legacy_inventory::inventory(directory, &manifest_digest, manifest)?
    } else {
        verify::inventory_of(&objects::directory(directory), &manifest_digest, manifest)?
    };
    let attempt = serialized_digest(&("pr9-policy-merge", &manifest_digest, format!("{state:?}")))?;
    let retained = Retained {
        attempt,
        manifest: manifest_digest,
    };
    root::write_retained(directory, &retained)?;
    if let Some(state) = state {
        install_historical_record(store, &retained.attempt, manifest, &inventory, state)?;
    }
    check!(eq; store.get_input_record(&retained.attempt)?.map(|record| record.state()), state);
    Ok(retained)
}

fn refused_prior_policy(store: &Store, job: &str, policy: u32) -> TestResult {
    check_refusal(freeze_v2(store, job), policy)
}

fn refused_prior_cancellation(store: &Store, job: &str, policy: u32) -> TestResult {
    check_refusal(cancel_freeze_v2(store, job), policy)
}

fn check_refusal<T>(result: Result<T, ReportError>, policy: u32) -> TestResult {
    if matches!(policy, 19..=20) {
        check!(matches!(
            result,
            Err(ReportError::Invariant { ref detail })
                if detail.contains("historical recruiting policy is read-only")
        ));
    } else {
        let expected = format!("policy revision {policy}");
        check!(matches!(
            result,
            Err(ReportError::ArtifactCorrupt { ref detail, .. })
                if (detail.contains("unsupported manifest lineage") && detail.contains(&expected))
                    || detail.contains("unsupported frozen manifest: census-frozen-jsonl-v2")
        ));
    }
    Ok(())
}

fn policy_preserved(policy: u32, state: Option<InputState>) -> TestResult {
    let seeded = schema_three::seed(policy)?;
    let store = seeded.store;
    let job = format!("pr9-policy-merge:{policy}:{state:?}");
    let current = freeze_v2(&store, &job)?;
    let directory = current.directory().to_path_buf();
    let prior = prior_manifest(current.manifest(), policy)?;
    let retained = install_prior_policy(&store, &directory, &prior, state)?;
    let before =
        RetainedEvidence::capture(&store, &directory, &[current.attempt(), &retained.attempt])?;
    (0..2).try_for_each(|_| -> TestResult {
        refused_prior_policy(&store, &job, policy)?;
        before.unchanged(&store, &directory)?;
        refused_prior_cancellation(&store, &job, policy)?;
        before.unchanged(&store, &directory)
    })?;
    drop(store);
    let reopened = Store::open(seeded.directory.path())?;
    before.unchanged(&reopened, &directory)?;
    (0..2).try_for_each(|_| -> TestResult {
        refused_prior_policy(&reopened, &job, policy)?;
        before.unchanged(&reopened, &directory)?;
        refused_prior_cancellation(&reopened, &job, policy)?;
        before.unchanged(&reopened, &directory)
    })?;
    check!(eq; root::read_retained(&directory)?, Some(retained));
    Ok(())
}

#[test]
fn historical_policies_eight_through_twenty_are_preserved_in_every_retained_state() -> TestResult {
    PRIOR_POLICIES.into_iter().try_for_each(|policy| {
        PRIOR_STATES
            .into_iter()
            .try_for_each(|state| policy_preserved(policy, state))
    })
}

#[derive(Clone, Copy)]
enum PriorCoverage {
    Captured,
    Missing,
}

fn prior_archive(
    dataset: &ExportDataset,
    store: &Store,
    policy: u32,
    coverage: PriorCoverage,
) -> TestResult<Vec<u8>> {
    if matches!(policy, 14..=20) {
        let data = serialized_bytes(dataset)?;
        let mut lineage = dataset.lineage.clone();
        lineage.schema_revision = 3;
        lineage.policy_revision = policy;
        lineage.input_digest = sha256_hex(&data);
        lineage.input_generation = generation(&lineage)?;
        return Ok(format!(
            "{{\"lineage\":{},\"data\":{}}}",
            String::from_utf8(serialized_bytes(&lineage)?)?,
            String::from_utf8(data)?
        )
        .into_bytes());
    }
    check!(dataset.relays.is_empty());
    let encoded = serialized_bytes(dataset)?;
    let prefix = encoded
        .strip_suffix(b",\"relays\":[]}")
        .ok_or("current relay suffix missing")?;
    let encoded = prefix
        .iter()
        .copied()
        .chain(std::iter::once(b'}'))
        .collect::<Vec<_>>();
    let mut lineage = dataset.lineage.clone();
    lineage.schema_revision = 2;
    lineage.source_digest = store
        .snapshot()
        .tables_digest(INPUT_TABLES.get(..15).ok_or("prior tables missing")?)?;
    lineage.policy_revision = policy;
    let data = match coverage {
        PriorCoverage::Captured => encoded,
        PriorCoverage::Missing => {
            let suffix = format!(
                ",\"coverage_evidence\":{}}}",
                String::from_utf8(serialized_bytes(&dataset.coverage_evidence)?)?
            );
            let prefix = encoded
                .strip_suffix(suffix.as_bytes())
                .ok_or("required coverage suffix missing")?;
            lineage.source_digest = store
                .snapshot()
                .tables_digest(INPUT_TABLES.get(..13).ok_or("prior tables missing")?)?;
            prefix
                .iter()
                .copied()
                .chain(std::iter::once(b'}'))
                .collect::<Vec<_>>()
        }
    };
    lineage.input_digest = sha256_hex(&data);
    lineage.input_generation = generation(&lineage)?;
    Ok(format!(
        "{{\"lineage\":{},\"data\":{}}}",
        String::from_utf8(serialized_bytes(&lineage)?)?,
        String::from_utf8(data)?
    )
    .into_bytes())
}

fn archive_preserved(policy: u32, coverage: PriorCoverage) -> TestResult {
    let seeded = schema_three::seed(policy)?;
    let store = seeded.store;
    let dataset = ExportDataset::load(&store)?;
    if matches!(policy, 15..=20) {
        check!(eq; dataset.relays.len(), 1);
        check!(eq; dataset.coverage_evidence.rows.len(), 1);
        check!(eq; dataset.coverage_evidence.coach_lanes.len(), 1);
    }
    let retained = prior_archive(&dataset, &store, policy, coverage)?;
    if matches!(policy, 14..=20) {
        let shape: serde_json::Value = serde_json::from_slice(&retained)?;
        check!(eq; shape.pointer("/lineage/schema_revision"), Some(&serde_json::json!(3)));
        check!(eq; shape.pointer("/lineage/policy_revision"), Some(&serde_json::json!(policy)));
        check!(eq; shape.pointer("/data/relays"), Some(&serde_json::to_value(&dataset.relays)?));
        check!(eq; shape.pointer("/data/coverage_evidence"), Some(&serde_json::to_value(&dataset.coverage_evidence)?));
    }
    let path = seeded.directory.path().join("prior-policy.json");
    std::fs::write(&path, &retained)?;
    let sequence = store.snapshot().sequence();
    (0..2).try_for_each(|_| -> TestResult {
        check_archive(&path, &store, policy, &retained)?;
        check!(eq; std::fs::read(&path)?, retained);
        check!(eq; store.snapshot().sequence(), sequence);
        Ok(())
    })?;
    drop(dataset);
    drop(store);
    let reopened = Store::open(seeded.directory.path())?;
    check_archive(&path, &reopened, policy, &retained)?;
    check!(eq; std::fs::read(&path)?, retained);
    check!(eq; reopened.snapshot().sequence(), sequence);
    Ok(())
}

#[test]
fn historical_full_coverage_archives_are_preserved_without_rewriting_their_bytes() -> TestResult {
    PRIOR_POLICIES
        .into_iter()
        .try_for_each(|policy| archive_preserved(policy, PriorCoverage::Captured))
}

fn check_archive(path: &Path, store: &Store, policy: u32, retained: &[u8]) -> TestResult {
    if matches!(policy, 19..=20) {
        let dataset = ExportDataset::reopen_frozen(path)?;
        check!(eq; dataset.lineage.policy_revision, policy);
        check!(eq; dataset.relays.len(), 1);
        check!(eq; dataset.coverage_evidence.rows.len(), 1);
        check!(eq; dataset.coverage_evidence.coach_lanes.len(), 1);
        check_refusal(dataset.ensure_current_policy(), policy)?;
        check_refusal(dataset.ensure_current(store), policy)?;
        let value: serde_json::Value = serde_json::from_slice(retained)?;
        check!(eq; serde_json::to_value(&dataset)?, *value.get("data").ok_or("historical data missing")?);
    } else {
        check!(matches!(
            ExportDataset::reopen_frozen(path),
            Err(ReportError::Invariant { .. })
        ));
    }
    Ok(())
}

#[test]
fn main_policy_eight_archive_is_refused_before_missing_coverage_decode() -> TestResult {
    archive_preserved(8, PriorCoverage::Missing)
}

fn main_style_manifest(
    store: &Store,
    current: &super::super::FrozenRoot,
) -> TestResult<FrozenManifest> {
    let dataset = super::super::to_export_dataset(current, store)?;
    check!(eq; dataset.coverage_evidence.rows.len(), 0);
    check!(eq; dataset.coverage_evidence.coach_lanes.len(), 0);
    let archive = prior_archive(&dataset, store, 8, PriorCoverage::Missing)?;
    let value: serde_json::Value = serde_json::from_slice(&archive)?;
    let lineage = value.get("lineage").ok_or("prior lineage missing")?.clone();
    let mut prior = current.manifest().clone();
    prior.lineage = serde_json::from_value(lineage)?;
    prior.legacy_logical_digest = Some(prior.lineage.input_digest.clone());
    prior.input_token.logical_schema = prior.lineage.schema_revision;
    prior.input_token.semantic_policy = 8;
    prior.input_token.input_root = prior.lineage.source_digest.clone();
    prior.streams.truncate(12);
    prior.source_fingerprints.truncate(13);
    prior.input_token.validate_against(&prior.lineage)?;
    check!(eq; prior.lineage.schema_revision, 2);
    check!(eq; prior.lineage.policy_revision, 8);
    check!(eq; prior.streams.len(), 12);
    check!(eq; prior.source_fingerprints.len(), 13);
    legacy_inventory::downgrade(current.directory(), &mut prior)?;
    Ok(prior)
}

fn main_style_preserved(state: Option<InputState>) -> TestResult {
    let seeded = seeded_store()?;
    let store = seeded.store;
    let job = format!("pr9-policy-merge:main-missing-coverage:{state:?}");
    let current = freeze_v2(&store, &job)?;
    let directory = current.directory().to_path_buf();
    let prior = main_style_manifest(&store, &current)?;
    let retained = install_prior_policy(&store, &directory, &prior, state)?;
    let before =
        RetainedEvidence::capture(&store, &directory, &[current.attempt(), &retained.attempt])?;
    (0..2).try_for_each(|_| -> TestResult {
        refused_prior_policy(&store, &job, 8)?;
        before.unchanged(&store, &directory)
    })?;
    drop(store);
    let reopened = Store::open(seeded.directory.path())?;
    (0..2).try_for_each(|_| -> TestResult {
        refused_prior_policy(&reopened, &job, 8)?;
        before.unchanged(&reopened, &directory)
    })?;
    check!(eq; root::read_retained(&directory)?, Some(retained));
    Ok(())
}

#[test]
fn main_policy_eight_missing_coverage_manifest_is_refused_without_retirement() -> TestResult {
    PRIOR_STATES.into_iter().try_for_each(main_style_preserved)
}
