use super::super::chunks::ChunkWriter;
use super::super::fixtures::{seeded_store, Bundle, TestResult};
use super::super::legacy::LegacyLogicalDigest;
use super::super::references::{ManifestFingerprint, ManifestStream};
use super::super::stream::StreamKind;
use super::super::token::FrozenInputToken;
use super::super::INPUT_TABLES;
use super::FrozenManifest;
use crate::export::dataset::ExportDataset;
use census_domain::model::{CanonicalSchool, CanonicalTeam};
use census_store::{Entity, Store, StoreSnapshot};

pub(super) struct Streams {
    pub(super) entries: Vec<(StreamKind, Bundle)>,
    pub(super) legacy: String,
}

pub(super) struct Fixture {
    pub(super) _directory: tempfile::TempDir,
    pub(super) store: Store,
    pub(super) dataset: ExportDataset,
    pub(super) streams: Streams,
    pub(super) manifest: FrozenManifest,
    pub(super) encoded: Vec<u8>,
}

impl Streams {
    pub(super) fn find(&self, kind: StreamKind) -> TestResult<&Bundle> {
        self.entries
            .iter()
            .find(|(entry, _)| *entry == kind)
            .map(|(_, bundle)| bundle)
            .ok_or_else(|| format!("{} stream is missing from the fixture", kind.wire()).into())
    }
}

pub(super) fn fixture() -> TestResult<Fixture> {
    let seeded = seeded_store()?;
    let store = seeded.store;
    store.store_identity()?;
    let (dataset, streams, manifest, encoded) = {
        let snapshot = store.snapshot();
        let dataset = ExportDataset::from_snapshot(&store, &snapshot)?;
        let streams = streams_of(&dataset)?;
        let (manifest, encoded) = manifest_of(&snapshot, &dataset, &streams)?;
        (dataset, streams, manifest, encoded)
    };
    Ok(Fixture {
        _directory: seeded.directory,
        store,
        dataset,
        streams,
        manifest,
        encoded,
    })
}

fn streams_of(dataset: &ExportDataset) -> TestResult<Streams> {
    let mut legacy = LegacyLogicalDigest::new();
    let mut entries = Vec::new();
    let mut athletes = dataset.athletes.clone();
    push_stream(
        &mut entries,
        StreamKind::Athletes,
        &mut athletes,
        &mut legacy,
    )?;
    let mut schools: Vec<CanonicalSchool> = dataset.schools.values().cloned().collect();
    push_stream(&mut entries, StreamKind::Schools, &mut schools, &mut legacy)?;
    let mut teams: Vec<CanonicalTeam> = dataset.teams.values().cloned().collect();
    push_stream(&mut entries, StreamKind::Teams, &mut teams, &mut legacy)?;
    let mut coaches = dataset.coaches.clone();
    push_stream(&mut entries, StreamKind::Coaches, &mut coaches, &mut legacy)?;
    let mut observations = dataset.coach_observations.clone();
    push_stream(
        &mut entries,
        StreamKind::CoachObservations,
        &mut observations,
        &mut legacy,
    )?;
    let mut events = dataset.events.clone();
    push_stream(&mut entries, StreamKind::Events, &mut events, &mut legacy)?;
    let mut meets = dataset.meets.clone();
    push_stream(&mut entries, StreamKind::Meets, &mut meets, &mut legacy)?;
    let mut performances = dataset.performances.clone();
    push_stream(
        &mut entries,
        StreamKind::Performances,
        &mut performances,
        &mut legacy,
    )?;
    let mut cases = dataset.review_cases.clone();
    push_stream(
        &mut entries,
        StreamKind::ReviewCases,
        &mut cases,
        &mut legacy,
    )?;
    let mut access = dataset.source_access.clone();
    push_stream(
        &mut entries,
        StreamKind::SourceAccess,
        &mut access,
        &mut legacy,
    )?;
    let mut verdicts = dataset.verdicts.clone();
    push_stream(
        &mut entries,
        StreamKind::Verdicts,
        &mut verdicts,
        &mut legacy,
    )?;
    let mut decisions = dataset.identity_decisions.clone();
    push_stream(
        &mut entries,
        StreamKind::IdentityDecisions,
        &mut decisions,
        &mut legacy,
    )?;
    let mut coverage = dataset.coverage_evidence.rows.clone();
    push_stream(
        &mut entries,
        StreamKind::CoverageRows,
        &mut coverage,
        &mut legacy,
    )?;
    let mut lanes = dataset.coverage_evidence.coach_lanes.clone();
    push_stream(
        &mut entries,
        StreamKind::CoachLanes,
        &mut lanes,
        &mut legacy,
    )?;
    let mut relays = dataset.relays.clone();
    push_stream(&mut entries, StreamKind::Relays, &mut relays, &mut legacy)?;
    let legacy = legacy.finish()?;
    Ok(Streams { entries, legacy })
}

fn push_stream<T: Entity>(
    entries: &mut Vec<(StreamKind, Bundle)>,
    kind: StreamKind,
    rows: &mut [T],
    legacy: &mut LegacyLogicalDigest,
) -> TestResult<()> {
    entries.push((kind, write_stream(kind, rows, legacy)?));
    Ok(())
}

fn write_stream<T: Entity>(
    kind: StreamKind,
    rows: &mut [T],
    legacy: &mut LegacyLogicalDigest,
) -> TestResult<Bundle> {
    rows.sort_by(|left, right| left.entity_id().cmp(right.entity_id()));
    legacy.begin(kind)?;
    let mut writer = ChunkWriter::new(kind);
    let mut objects = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let ordinal = u64::try_from(index)?;
        legacy.push(kind, row)?;
        if let Some(chunk) = writer.push(ordinal, row)? {
            objects.push(chunk.bytes);
        }
    }
    legacy.close(kind)?;
    let (last, inventory) = writer.finish()?;
    if let Some(chunk) = last {
        objects.push(chunk.bytes);
    }
    Ok(Bundle { inventory, objects })
}

fn manifest_of(
    snapshot: &StoreSnapshot<'_>,
    dataset: &ExportDataset,
    streams: &Streams,
) -> TestResult<(FrozenManifest, Vec<u8>)> {
    let token = FrozenInputToken::capture(snapshot, &dataset.lineage)?;
    let mut references = Vec::new();
    for (_, bundle) in &streams.entries {
        let encoded = bundle.inventory.encode()?;
        references.push(ManifestStream::of(&bundle.inventory, &encoded)?);
    }
    let mut fingerprints = Vec::new();
    for table in INPUT_TABLES {
        let digest = snapshot.tables_digest(&[table])?;
        fingerprints.push(ManifestFingerprint::of(table, &digest)?);
    }
    let manifest = FrozenManifest::new(
        dataset.lineage.clone(),
        token,
        references,
        fingerprints,
        Some(streams.legacy.clone()),
    )?;
    let encoded = manifest.encode()?;
    Ok((manifest, encoded))
}
