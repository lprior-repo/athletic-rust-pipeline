use super::super::codec::{sha256_hex, FrozenCodecResult, CODEC_REVISION, TOKEN_PROTOCOL};
use super::super::fixtures::{refusal, refusal_what, TestResult};
use super::super::inventory::{ArtifactRef, StreamInventory};
use super::super::manifest::FrozenManifest;
use super::super::root::{self, FrozenRoot, Retained};
use super::super::stream::StreamKind;
use super::super::token::FrozenInputToken;
use super::super::{objects, to_export_dataset, verify, INPUT_TABLES};
use super::version_records::{install_record, RetainedState};
use census_domain::model::{serialized_bytes, serialized_digest};
use census_store::Store;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VersionCase {
    HistoricalSchema,
    HistoricalPolicy,
    PolicyThirteen,
    PolicyFourteen,
    CodecTwo,
    PreviousPolicy,
    PriorAffiliationPolicy,
    PriorParentScopePolicy,
    HistoricalProtocol,
    ManifestCodec,
    AdvertisedInventory,
    InventoryHeader,
    ChunkDescriptor,
    Supported,
}

impl VersionCase {
    pub(super) fn refusal_what(self) -> &'static str {
        match self {
            Self::HistoricalSchema
            | Self::HistoricalPolicy
            | Self::PolicyThirteen
            | Self::PolicyFourteen
            | Self::PreviousPolicy
            | Self::PriorAffiliationPolicy
            | Self::PriorParentScopePolicy => "manifest lineage",
            Self::HistoricalProtocol => "input token",
            Self::ManifestCodec | Self::CodecTwo => "frozen manifest",
            Self::AdvertisedInventory => "artifact reference",
            Self::InventoryHeader => "stream inventory",
            Self::ChunkDescriptor => "chunk descriptor",
            Self::Supported => "supported fixture",
        }
    }
}

pub(super) fn install(
    store: &Store,
    current: &FrozenRoot,
    case: VersionCase,
    state: RetainedState,
) -> TestResult<Retained> {
    let mut manifest = current.manifest().clone();
    let excluded = match case {
        VersionCase::HistoricalSchema => historical_schema(store, current, &mut manifest)?,
        _ => Vec::new(),
    };
    change_manifest(&mut manifest, case)?;
    let replacement = change_page(current.directory(), &mut manifest, case)?;
    let encoded = serialized_bytes(&manifest)?;
    check_manifest(&manifest, &encoded, case)?;
    let manifest_digest = install_bytes(current.directory(), &encoded)?;
    let original = store
        .get_input_record(current.attempt())?
        .ok_or("the supported baseline input is missing")?;
    let inventory = original
        .chunks()
        .iter()
        .filter(|digest| !excluded.contains(*digest))
        .map(|digest| replacement_digest(digest, current, &manifest_digest, &replacement))
        .collect::<Vec<_>>();
    let attempt = serialized_digest(&(
        "version-preservation-fixture",
        format!("{case:?}:{state:?}"),
        &manifest_digest,
    ))?;
    let retained = Retained {
        attempt,
        manifest: manifest_digest,
    };
    let bytes = root::payload_bytes_of(&manifest)?;
    check!(eq; bytes, original.bytes());
    if let Some(record) = install_record(store, &retained.attempt, bytes, &inventory, state)? {
        verify::check_record(&manifest, &record, &inventory)?;
    }
    root::write_retained(current.directory(), &retained)?;
    Ok(retained)
}

fn historical_schema(
    store: &Store,
    current: &FrozenRoot,
    manifest: &mut FrozenManifest,
) -> TestResult<Vec<String>> {
    let dataset = to_export_dataset(current, store)?;
    check!(eq; dataset.coverage_evidence.rows.len(), 0);
    check!(eq; dataset.coverage_evidence.coach_lanes.len(), 0);
    check!(eq; manifest.streams.len(), 15);
    check!(eq; manifest.source_fingerprints.len(), 16);
    let encoded = serialized_bytes(&dataset)?;
    let suffix = br#","coverage_evidence":{"rows":[],"coach_lanes":[]},"relays":[]}"#;
    let prefix = encoded
        .strip_suffix(suffix)
        .ok_or("the canonical coverage suffix is missing")?;
    let historical = prefix
        .iter()
        .copied()
        .chain(std::iter::once(b'}'))
        .collect::<Vec<_>>();
    let historical_digest = sha256_hex(&historical);
    check!(ne; historical_digest, manifest.lineage.input_digest);
    manifest.lineage.input_digest = historical_digest.clone();
    manifest.legacy_logical_digest = Some(historical_digest);
    let old_tables = INPUT_TABLES
        .get(..13)
        .ok_or("historical input tables missing")?;
    manifest.lineage.source_digest = store.snapshot().tables_digest(old_tables)?;
    manifest.input_token.input_root = manifest.lineage.source_digest.clone();
    let excluded = [
        StreamKind::CoverageRows,
        StreamKind::CoachLanes,
        StreamKind::Relays,
    ]
    .into_iter()
    .map(|kind| -> TestResult<_> {
        let reference = manifest.stream(kind).ok_or("coverage stream missing")?;
        check!(eq; reference.rows, 0);
        check!(eq; reference.bytes, 0);
        Ok(reference.inventory.digest.clone())
    })
    .collect::<TestResult<Vec<_>>>()?;
    manifest.streams.truncate(12);
    manifest.source_fingerprints.truncate(13);
    check!(eq; manifest.streams.len(), 12);
    check!(eq; manifest.source_fingerprints.len(), 13);
    Ok(excluded)
}

fn change_manifest(manifest: &mut FrozenManifest, case: VersionCase) -> TestResult {
    match case {
        VersionCase::HistoricalSchema => {
            check!(eq; manifest.lineage.schema_revision, 3);
            manifest.lineage.schema_revision = 1;
            manifest.input_token.logical_schema = 1;
            manifest.lineage.policy_revision = 8;
            manifest.input_token.semantic_policy = 8;
        }
        VersionCase::HistoricalPolicy => {
            check!(eq; manifest.lineage.policy_revision, 15);
            manifest.lineage.policy_revision = 7;
            manifest.input_token.semantic_policy = 7;
        }
        VersionCase::PolicyFourteen => {
            check!(eq; manifest.lineage.schema_revision, 3);
            check!(eq; manifest.lineage.policy_revision, 15);
            manifest.lineage.policy_revision = 14;
            manifest.input_token.semantic_policy = 14;
        }
        VersionCase::PolicyThirteen => {
            check!(eq; manifest.lineage.schema_revision, 3);
            check!(eq; manifest.lineage.policy_revision, 15);
            manifest.lineage.schema_revision = 2;
            manifest.lineage.policy_revision = 13;
            manifest.input_token.logical_schema = 2;
            manifest.input_token.semantic_policy = 13;
        }
        VersionCase::CodecTwo => {
            manifest.codec = "census-frozen-jsonl-v2".to_string();
            manifest.codec_revision = 2;
        }
        VersionCase::PreviousPolicy => {
            check!(eq; manifest.lineage.policy_revision, 15);
            manifest.lineage.policy_revision = 8;
            manifest.input_token.semantic_policy = 8;
        }
        VersionCase::PriorAffiliationPolicy => {
            check!(eq; manifest.lineage.policy_revision, 15);
            manifest.lineage.policy_revision = 9;
            manifest.input_token.semantic_policy = 9;
        }
        VersionCase::PriorParentScopePolicy => {
            check!(eq; manifest.lineage.policy_revision, 15);
            manifest.lineage.policy_revision = 10;
            manifest.input_token.semantic_policy = 10;
        }
        VersionCase::HistoricalProtocol => {
            check!(eq; manifest.input_token.protocol, TOKEN_PROTOCOL);
            manifest.input_token.protocol = 1;
        }
        VersionCase::ManifestCodec => manifest.codec_revision = future_codec_revision()?,
        VersionCase::AdvertisedInventory
        | VersionCase::InventoryHeader
        | VersionCase::ChunkDescriptor
        | VersionCase::Supported => {}
    }
    manifest.lineage.input_generation = super::super::generation(&manifest.lineage)?;
    FrozenInputToken {
        protocol: TOKEN_PROTOCOL,
        ..manifest.input_token.clone()
    }
    .validate_against(&manifest.lineage)?;
    Ok(())
}

fn change_page(
    directory: &Path,
    manifest: &mut FrozenManifest,
    case: VersionCase,
) -> TestResult<Option<(String, String)>> {
    if !matches!(
        case,
        VersionCase::AdvertisedInventory
            | VersionCase::InventoryHeader
            | VersionCase::ChunkDescriptor
    ) {
        return Ok(None);
    }
    let reference = manifest
        .streams
        .get_mut(StreamKind::Schools.index())
        .ok_or("the schools stream is missing")?;
    let old_digest = reference.inventory.digest.clone();
    let encoded = objects::read(
        &objects::directory(directory),
        &old_digest,
        objects::PAGE_CEILING,
    )?;
    let mut page = StreamInventory::decode(&encoded)?;
    let descriptor = page
        .chunks
        .first_mut()
        .ok_or("the schools chunk is missing")?;
    check!(eq; descriptor.codec_revision, CODEC_REVISION);
    if case == VersionCase::ChunkDescriptor {
        descriptor.codec_revision = future_codec_revision()?;
    } else {
        page.codec_revision = future_codec_revision()?;
    }
    let encoded = serialized_bytes(&page)?;
    let what = if case == VersionCase::ChunkDescriptor {
        "chunk descriptor"
    } else {
        "stream inventory"
    };
    check_unsupported(StreamInventory::decode(&encoded), what)?;
    reference.inventory = ArtifactRef::of(&encoded)?;
    if case == VersionCase::AdvertisedInventory {
        reference.inventory.codec_revision = future_codec_revision()?;
    }
    let digest = install_bytes(directory, &encoded)?;
    check!(eq; reference.inventory.digest, digest);
    check!(eq; reference.rows, page.rows);
    check!(eq; reference.bytes, page.bytes);
    Ok(Some((old_digest, digest)))
}

fn check_manifest(manifest: &FrozenManifest, encoded: &[u8], case: VersionCase) -> TestResult {
    match case {
        VersionCase::InventoryHeader | VersionCase::ChunkDescriptor | VersionCase::Supported => {
            check!(eq; FrozenManifest::decode(encoded)?, *manifest);
        }
        _ => check_unsupported(FrozenManifest::decode(encoded), case.refusal_what())?,
    }
    Ok(())
}

fn check_unsupported<T>(result: FrozenCodecResult<T>, what: &'static str) -> TestResult {
    let error = refusal(result)?;
    check!(matches!(
        &error,
        super::super::codec::FrozenCodecError::UnsupportedVersion { .. }
    ));
    check!(eq; refusal_what(&error), Some(what));
    Ok(())
}

fn install_bytes(directory: &Path, encoded: &[u8]) -> TestResult<String> {
    let objects_dir = objects::directory(directory);
    let digest = objects::install(&objects_dir, encoded)?;
    check!(eq; objects::read(&objects_dir, &digest, objects::PAGE_CEILING)?, encoded);
    Ok(digest)
}

fn replacement_digest(
    digest: &str,
    current: &FrozenRoot,
    manifest_digest: &str,
    page: &Option<(String, String)>,
) -> String {
    if digest == current.manifest_digest() {
        return manifest_digest.to_string();
    }
    match page {
        Some((old, new)) if digest == old => new.clone(),
        _ => digest.to_string(),
    }
}

fn future_codec_revision() -> TestResult<u32> {
    CODEC_REVISION
        .checked_add(1)
        .ok_or_else(|| "fixture codec revision overflow".into())
}
