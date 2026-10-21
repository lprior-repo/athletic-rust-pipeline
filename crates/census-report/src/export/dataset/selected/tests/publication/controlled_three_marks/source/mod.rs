use census_crawl::milesplit::SourceRowLocator;
use census_crawl::participation_evidence::provider_school::{
    ProviderSchoolPacketRefV1, ProviderSchoolPacketV1,
};
use census_crawl::participation_evidence::provider_target::SourceTargetPacketV1;
use census_crawl::participation_evidence::recorded::{
    record_provider_school_intake, record_provider_target_intake, ProviderTargetIntakeDispositionV1,
};
use census_crawl::participation_evidence::RetainedHtmlCaptureV1;
use census_crawl::Recording;
use census_domain::model::{AthleteCandidateId, SchoolId};
use census_domain::public_identity_history::PublicAthleteId;
use census_store::{JournalReadBudget, ReadBudget, RetainedCaptureRootSchemaV1, Store};
use chrono::{DateTime, Utc};
use serde_json::Value;

mod results;
mod target;
pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const DIRECTORY: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../census-crawl/src/participation_evidence/provider_school/fixtures/mi-team-index.html"
));
const ROSTER: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../census-crawl/src/participation_evidence/provider_school/fixtures/dcc-provider-roster.html"
));
const DIRECTORY_URL: &str = "https://mi.milesplit.com/teams";
const ROSTER_URL: &str = "https://mi.milesplit.com/teams/12439-detroit-catholic-central/roster";
const PROFILE_URL: &str = "https://mi.milesplit.com/athletes/9999999-controlled-target";
const RESULTS_URL: &str =
    "https://mi.milesplit.com/meets/999999-controlled-championships/teams/12439";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ControlledMark {
    pub(super) time: u16,
    pub(super) date: &'static str,
}

pub(super) const MARKS: [ControlledMark; 3] = [
    ControlledMark {
        time: 4900,
        date: "2026-02-28",
    },
    ControlledMark {
        time: 5000,
        date: "2026-05-15",
    },
    ControlledMark {
        time: 5100,
        date: "2026-05-16",
    },
];

#[derive(Debug, Clone)]
pub(super) struct SourceReference {
    pub(super) schema: RetainedCaptureRootSchemaV1,
    pub(super) key: String,
    pub(super) payload: Value,
}

#[derive(Debug, Clone)]
pub(super) struct RowLocator {
    pub(super) document_key: String,
    pub(super) locator: SourceRowLocator,
}

pub(super) struct InstalledSource {
    pub(super) directory: tempfile::TempDir,
    pub(super) store: Store,
    pub(super) identity_writer: census_store::PublicIdentityWriterV1,
    pub(super) school: SchoolId,
    pub(super) public: PublicAthleteId,
    pub(super) candidate: AthleteCandidateId,
    pub(super) source_records: Vec<SourceReference>,
    pub(super) row_locators: Vec<RowLocator>,
}

pub(super) struct CurrentSchoolInput {
    pub(super) school: SchoolId,
    pub(super) reference: ProviderSchoolPacketRefV1,
}

pub(super) fn capture(
    url: &str,
    body: &[u8],
    headers: &[u8],
    started: &str,
    ended: &str,
) -> TestResult<RetainedHtmlCaptureV1> {
    Ok(RetainedHtmlCaptureV1::new(
        url,
        url,
        200,
        DateTime::parse_from_rfc3339(ended)?.with_timezone(&Utc),
        body,
        census_crawl::participation_evidence::RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: vec![
                ("User-Agent".into(), "controlled-test/0.1".into()),
                ("Accept".into(), "text/html".into()),
            ],
            response_header_bytes: headers.to_vec(),
            content_type: "text/html; charset=UTF-8".into(),
            body_encoding: census_crawl::participation_evidence::RetainedBodyEncodingV1::Identity,
            started_at: DateTime::parse_from_rfc3339(started)?.with_timezone(&Utc),
        },
    )?)
}

fn school_packet() -> TestResult<ProviderSchoolPacketV1> {
    let directory = capture(
        DIRECTORY_URL,
        DIRECTORY,
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../census-crawl/src/participation_evidence/provider_school/fixtures/mi-team-index.headers")),
        "2026-10-16T02:57:54Z",
        "2026-10-16T02:58:00Z",
    )?;
    let roster = capture(
        ROSTER_URL,
        ROSTER,
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../census-crawl/src/participation_evidence/provider_school/fixtures/dcc-provider-roster.headers")),
        "2026-10-16T00:49:38Z",
        "2026-10-16T00:49:43Z",
    )?;
    Ok(ProviderSchoolPacketV1::new(directory, roster)?)
}

fn school_budget() -> ReadBudget {
    ReadBudget {
        rows: 65_536,
        bytes: 32 * 1024 * 1024,
        record_bytes: 2 * 1024 * 1024,
    }
}

fn protected_state(store: &Store) -> TestResult<(u64, u64, u64)> {
    let snapshot = store.snapshot();
    Ok((
        snapshot.evidence_generation(),
        snapshot.derived_generation(),
        store.receipt_count()?,
    ))
}

pub(super) fn require_originals(store: &Store, sources: &[SourceReference]) -> TestResult {
    let snapshot = store.snapshot();
    for source in sources {
        let raw = snapshot
            .journal_payload_raw_bounded(
                source.schema.phase(),
                &source.key,
                JournalReadBudget::new(2 * 1024 * 1024)?,
            )?
            .ok_or("original root absent")?;
        let payload: Value = serde_json::from_str(raw.get())?;
        if payload != source.payload {
            return Err("original payload mismatch".into());
        }
    }
    Ok(())
}

fn enroll_school(store: &Store) -> TestResult<(SchoolId, SourceReference)> {
    let school_packet = school_packet()?;
    let school_reference = school_packet.reference()?;
    let school_payload = serde_json::to_value(&school_packet)?;
    let recording = Recording::new();
    let before = protected_state(store)?;
    let school = record_provider_school_intake(store, &recording, &school_packet)?;
    let after = protected_state(store)?;
    if before != after {
        return Err("school intake changed state".into());
    }
    let recorded = recording.drain();
    let prepared = census_crawl::recording::prepare_recorded(recorded, "controlled-school-intake")?;
    prepared.apply(store)?;
    let source_reference = SourceReference {
        schema: RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1,
        key: school_reference.as_str().to_owned(),
        payload: school_payload,
    };
    Ok((school, source_reference))
}

fn enroll_target(store: &Store) -> TestResult<(AthleteCandidateId, SourceReference)> {
    let target = target::packet()?;
    let target_reference = target.reference()?;
    let target_payload = serde_json::to_value(&target)?;
    let recording = Recording::new();
    let before = protected_state(store)?;
    let ProviderTargetIntakeDispositionV1::ProjectedUnaccepted { candidate } =
        record_provider_target_intake(store, &recording, &target, school_budget())?
    else {
        return Err("source target was not projected".into());
    };
    let after = protected_state(store)?;
    if before != after {
        return Err("target intake changed state".into());
    }
    let recorded = recording.drain();
    let prepared = census_crawl::recording::prepare_recorded(recorded, "controlled-target-intake")?;
    prepared.apply(store)?;
    let source_reference = SourceReference {
        schema: RetainedCaptureRootSchemaV1::ProviderTargetPacketV1,
        key: target_reference.as_str().to_owned(),
        payload: target_payload,
    };
    Ok((candidate.cast(), source_reference))
}

pub(super) fn install() -> TestResult<InstalledSource> {
    install_with_result_venue("Controlled Track Venue")
}

pub(super) fn install_with_result_venue(venue: &str) -> TestResult<InstalledSource> {
    install_with_marks(venue, &MARKS)
}

pub(super) fn install_with_marks(
    venue: &str,
    marks: &[ControlledMark],
) -> TestResult<InstalledSource> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("store");
    Store::create_public_identity_qualification_v3(&path)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&path)?.into_parts();
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;

    let (school, school_source) = enroll_school(&store)?;
    let (candidate, target_source) = enroll_target(&store)?;
    let (result_sources, row_locators) = results::install_with_marks(&store, venue, marks)?;

    let mut source_records = vec![school_source, target_source];
    source_records.extend(result_sources);

    let birth = writer.establish_source_bound(
        &store,
        "controlled-source-bound-target",
        &candidate,
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    let public = birth.public_id().clone();

    require_originals(&store, &source_records)?;

    Ok(InstalledSource {
        directory,
        store,
        identity_writer: writer,
        school,
        public,
        candidate,
        source_records,
        row_locators,
    })
}

pub(super) fn install_with_marks(marks: &[ControlledMark]) -> TestResult<InstalledSource> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("store");
    Store::create_public_identity_qualification_v3(&path)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&path)?.into_parts();
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;

    let (school, school_source) = enroll_school(&store)?;
    let (candidate, target_source) = enroll_target(&store)?;
    let (result_sources, row_locators) =
        results::install_with_marks(&store, "Controlled Track Venue", marks)?;

    let mut source_records = vec![school_source, target_source];
    source_records.extend(result_sources);

    let birth = writer.establish_source_bound(
        &store,
        "controlled-source-bound-target",
        &candidate,
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    let public = birth.public_id().clone();

    require_originals(&store, &source_records)?;

    Ok(InstalledSource {
        directory,
        store,
        identity_writer: writer,
        school,
        public,
        candidate,
        source_records,
        row_locators,
    })
}

pub(super) fn require_identity(
    store: &Store,
    candidate: &AthleteCandidateId,
    expected_public: &PublicAthleteId,
) -> TestResult {
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("current identity ledger")?;
    let resolution = accepted.resolve_candidate(candidate)?;
    match resolution {
        census_domain::public_identity_history::PublicResolution::Canonical(row) => {
            if row.public_id != *expected_public {
                return Err("resolved public id mismatch".into());
            }
        }
        other => {
            return Err(format!("expected canonical resolution, got {:?}", other).into());
        }
    }
    Ok(())
}
