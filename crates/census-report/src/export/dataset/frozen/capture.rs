use super::chunks::{ChunkObject, ChunkWriter};
use super::codec::count;
use super::legacy::LegacyLogicalDigest;
use super::manifest::FrozenManifest;
use super::objects;
use super::references::{ManifestFingerprint, ManifestStream};
use super::stream::StreamKind;
use super::token::FrozenInputToken;
use super::{codec_error, invalid, INPUT_TABLES};
use crate::report::{ReportError, ReportResult};
use census_domain::model::{
    AppliedAthleteIdentity, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, CoachLaneObligation, CoverageRow, RelayResult, ReviewCase,
    ReviewVerdictRecord, SourceAccessCondition,
};
use census_store::{Entity, Store, StoreError, StoreSnapshot, Table};
use std::path::Path;

pub(super) fn capture(
    store: &Store,
    job: &str,
    objects_dir: &Path,
) -> ReportResult<FrozenManifest> {
    store.store_identity()?;
    let snapshot = store.snapshot();
    let mut lineage = super::lineage(store, &snapshot)?;
    lineage.export_job = Some(job.to_string());
    let token = FrozenInputToken::capture(&snapshot, &lineage)?;
    let (streams, legacy) = streams(&snapshot, objects_dir)?;
    let fingerprints = fingerprints(&snapshot)?;
    drop(snapshot);
    lineage.input_digest = legacy.clone();
    lineage.input_generation = super::generation(&lineage)?;
    FrozenManifest::new(lineage, token, streams, fingerprints, Some(legacy)).map_err(codec_error)
}

fn streams(
    snapshot: &StoreSnapshot<'_>,
    objects_dir: &Path,
) -> ReportResult<(Vec<ManifestStream>, String)> {
    let mut sink = StreamSink::new(objects_dir);
    sink.athletes(snapshot)?;
    sink.merged::<CanonicalSchool>(StreamKind::Schools, Table::Schools, snapshot)?;
    sink.merged::<CanonicalTeam>(StreamKind::Teams, Table::Teams, snapshot)?;
    sink.merged::<CanonicalCoach>(StreamKind::Coaches, Table::Coaches, snapshot)?;
    sink.observed(snapshot)?;
    sink.merged::<CanonicalEvent>(StreamKind::Events, Table::Events, snapshot)?;
    sink.merged::<CanonicalMeet>(StreamKind::Meets, Table::Meets, snapshot)?;
    sink.merged::<CanonicalPerformance>(StreamKind::Performances, Table::Performances, snapshot)?;
    sink.merged::<ReviewCase>(StreamKind::ReviewCases, Table::ReviewCases, snapshot)?;
    sink.merged::<SourceAccessCondition>(StreamKind::SourceAccess, Table::SourceAccess, snapshot)?;
    sink.merged::<ReviewVerdictRecord>(StreamKind::Verdicts, Table::IdentityVerdicts, snapshot)?;
    sink.merged::<AppliedAthleteIdentity>(
        StreamKind::IdentityDecisions,
        Table::AthleteIdentityDecisions,
        snapshot,
    )?;
    sink.merged::<CoverageRow>(StreamKind::CoverageRows, Table::Coverage, snapshot)?;
    sink.merged::<CoachLaneObligation>(StreamKind::CoachLanes, Table::CoachLanes, snapshot)?;
    sink.merged::<RelayResult>(StreamKind::Relays, Table::RelayResults, snapshot)?;
    sink.finish()
}

fn fingerprints(snapshot: &StoreSnapshot<'_>) -> ReportResult<Vec<ManifestFingerprint>> {
    INPUT_TABLES
        .into_iter()
        .map(|table| {
            let digest = snapshot.tables_digest(&[table])?;
            ManifestFingerprint::of(table, &digest).map_err(codec_error)
        })
        .collect()
}

struct StreamSink<'a> {
    objects_dir: &'a Path,
    legacy: LegacyLogicalDigest,
    streams: Vec<ManifestStream>,
}

impl<'a> StreamSink<'a> {
    fn new(objects_dir: &'a Path) -> Self {
        Self {
            objects_dir,
            legacy: LegacyLogicalDigest::new(),
            streams: Vec::new(),
        }
    }

    fn athletes(&mut self, snapshot: &StoreSnapshot<'_>) -> ReportResult<()> {
        let rows = snapshot.athletes()?;
        let expected = count(rows.len()).map_err(codec_error)?;
        let mut writer =
            StreamWriter::new(StreamKind::Athletes, self.objects_dir, &mut self.legacy)?;
        for row in &rows {
            writer.accept(row)?;
        }
        let stream = writer.finish()?;
        self.push(stream, expected)
    }

    fn merged<T: Entity>(
        &mut self,
        kind: StreamKind,
        table: Table,
        snapshot: &StoreSnapshot<'_>,
    ) -> ReportResult<()> {
        let mut writer = StreamWriter::new(kind, self.objects_dir, &mut self.legacy)?;
        let mut failure: Option<ReportError> = None;
        let mut fold = fold::EntityFold::new(fold::ENCODED_GROUP_LIMIT);
        let mut rows = 0_u64;
        let mut accept = |row: T| {
            rows = rows.checked_add(1).ok_or(StoreError::CounterOverflow)?;
            writer.accept(&row).map_err(|error| {
                failure = Some(error);
                StoreError::Refused {
                    detail: "frozen record writer refused the canonical entity".to_string(),
                }
            })
        };
        let scanned = snapshot
            .for_each_observation(table, |row: T| fold.accept(row, &mut accept))
            .and_then(|()| fold.finish(&mut accept));
        if let Some(error) = failure {
            return Err(error);
        }
        scanned?;
        let stream = writer.finish()?;
        self.push(stream, rows)
    }

    fn observed(&mut self, snapshot: &StoreSnapshot<'_>) -> ReportResult<()> {
        let mut writer = StreamWriter::new(
            StreamKind::CoachObservations,
            self.objects_dir,
            &mut self.legacy,
        )?;
        let mut failure: Option<ReportError> = None;
        let mut rows: u64 = 0;
        snapshot.for_each_observation(Table::Coaches, |mut row: CanonicalCoach| {
            Entity::publish(&mut row);
            if failure.is_none() {
                failure = observe(&mut writer, &mut rows, &row).err();
            }
            Ok(())
        })?;
        if let Some(error) = failure {
            return Err(error);
        }
        let stream = writer.finish()?;
        self.push(stream, rows)
    }

    fn push(&mut self, stream: ManifestStream, expected: u64) -> ReportResult<()> {
        if stream.rows != expected {
            return Err(invalid(
                "a frozen stream holds a different row count than its capture scan",
            ));
        }
        self.streams.push(stream);
        Ok(())
    }

    fn finish(self) -> ReportResult<(Vec<ManifestStream>, String)> {
        let Self {
            legacy, streams, ..
        } = self;
        Ok((streams, legacy.finish().map_err(codec_error)?))
    }
}

fn observe(
    writer: &mut StreamWriter<'_>,
    rows: &mut u64,
    row: &CanonicalCoach,
) -> ReportResult<()> {
    writer.accept(row)?;
    *rows = rows
        .checked_add(1)
        .ok_or_else(|| invalid("the observation count overflowed u64"))?;
    Ok(())
}

struct StreamWriter<'a> {
    kind: StreamKind,
    objects_dir: &'a Path,
    legacy: &'a mut LegacyLogicalDigest,
    chunk: ChunkWriter,
    chunks: u64,
}

impl<'a> StreamWriter<'a> {
    fn new(
        kind: StreamKind,
        objects_dir: &'a Path,
        legacy: &'a mut LegacyLogicalDigest,
    ) -> ReportResult<Self> {
        legacy.begin(kind).map_err(codec_error)?;
        Ok(Self {
            kind,
            objects_dir,
            legacy,
            chunk: ChunkWriter::new(kind),
            chunks: 0,
        })
    }

    fn accept<T: Entity>(&mut self, row: &T) -> ReportResult<()> {
        let ordinal = self.chunk.rows();
        let sealed = self.chunk.push(ordinal, row).map_err(codec_error)?;
        self.legacy.push(self.kind, row).map_err(codec_error)?;
        match sealed {
            Some(object) => self.seal(object),
            None => Ok(()),
        }
    }

    fn seal(&mut self, object: ChunkObject) -> ReportResult<()> {
        seal_chunk(self.objects_dir, &mut self.chunks, object)
    }

    fn finish(self) -> ReportResult<ManifestStream> {
        let Self {
            kind,
            objects_dir,
            legacy,
            chunk,
            mut chunks,
        } = self;
        let (sealed, inventory) = chunk.finish().map_err(codec_error)?;
        if let Some(object) = sealed {
            seal_chunk(objects_dir, &mut chunks, object)?;
        }
        legacy.close(kind).map_err(codec_error)?;
        let encoded = inventory.encode().map_err(codec_error)?;
        let stream = ManifestStream::of(&inventory, &encoded).map_err(codec_error)?;
        if objects::install(objects_dir, &encoded)? != stream.inventory.digest {
            return Err(invalid(
                "a frozen inventory object disagrees with its digest name",
            ));
        }
        Ok(stream)
    }
}

fn seal_chunk(objects_dir: &Path, chunks: &mut u64, object: ChunkObject) -> ReportResult<()> {
    if object.index != *chunks {
        return Err(invalid("a frozen chunk was sealed out of capture order"));
    }
    *chunks = chunks
        .checked_add(1)
        .ok_or_else(|| invalid("the frozen chunk count overflowed u64"))?;
    objects::install(objects_dir, &object.bytes).map(|_| ())
}

#[cfg(test)]
#[path = "fold_tests.rs"]
mod fold_tests;

#[path = "fold.rs"]
mod fold;
