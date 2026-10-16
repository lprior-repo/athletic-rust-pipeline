use super::super::{CoverageEvidence, Loaded};
use super::manifest::FrozenManifest;
use super::objects;
use super::reader::ChunkReader;
use super::stream::StreamKind;
use super::{codec_error, invalid};
use crate::report::ReportResult;
use census_domain::model::{
    AppliedAthleteIdentity, CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet,
    CanonicalPerformance, CanonicalSchool, CanonicalTeam, CoachLaneObligation, CoverageRow,
    RelayResult, ReviewCase, ReviewVerdictRecord, SourceAccessCondition,
};
use census_store::Entity;
use std::path::Path;

pub(super) fn read_all(objects_dir: &Path, manifest: &FrozenManifest) -> ReportResult<Loaded> {
    Ok(Loaded {
        selection: None,
        athletes: collect::<CanonicalAthlete>(objects_dir, StreamKind::Athletes, manifest)?,
        schools: collect::<CanonicalSchool>(objects_dir, StreamKind::Schools, manifest)?,
        teams: collect::<CanonicalTeam>(objects_dir, StreamKind::Teams, manifest)?,
        coaches: collect::<CanonicalCoach>(objects_dir, StreamKind::Coaches, manifest)?,
        coach_observations: collect::<CanonicalCoach>(
            objects_dir,
            StreamKind::CoachObservations,
            manifest,
        )?,
        events: collect::<CanonicalEvent>(objects_dir, StreamKind::Events, manifest)?,
        meets: collect::<CanonicalMeet>(objects_dir, StreamKind::Meets, manifest)?,
        performances: collect::<CanonicalPerformance>(
            objects_dir,
            StreamKind::Performances,
            manifest,
        )?,
        review_cases: collect::<ReviewCase>(objects_dir, StreamKind::ReviewCases, manifest)?,
        source_access: collect::<SourceAccessCondition>(
            objects_dir,
            StreamKind::SourceAccess,
            manifest,
        )?,
        verdicts: collect::<ReviewVerdictRecord>(objects_dir, StreamKind::Verdicts, manifest)?,
        identity_decisions: collect::<AppliedAthleteIdentity>(
            objects_dir,
            StreamKind::IdentityDecisions,
            manifest,
        )?,
        relays: collect::<RelayResult>(objects_dir, StreamKind::Relays, manifest)?,
        coverage_evidence: CoverageEvidence {
            rows: collect::<CoverageRow>(objects_dir, StreamKind::CoverageRows, manifest)?,
            coach_lanes: collect::<CoachLaneObligation>(
                objects_dir,
                StreamKind::CoachLanes,
                manifest,
            )?,
        },
    })
}

fn collect<T: Entity>(
    objects_dir: &Path,
    kind: StreamKind,
    manifest: &FrozenManifest,
) -> ReportResult<Vec<T>> {
    let reference = manifest
        .stream(kind)
        .ok_or_else(|| invalid("a frozen manifest is missing a required stream"))?;
    let page = super::verify::page(objects_dir, reference)?;
    let mut rows = Vec::new();
    for descriptor in page.chunks() {
        let bytes = objects::read(objects_dir, &descriptor.sha256, objects::CHUNK_CEILING)?;
        let mut reader =
            ChunkReader::open(kind, &bytes, descriptor.first_ordinal).map_err(codec_error)?;
        while let Some(decoded) = reader.next_record::<T>().map_err(codec_error)? {
            rows.push(decoded.payload);
        }
    }
    Ok(rows)
}
