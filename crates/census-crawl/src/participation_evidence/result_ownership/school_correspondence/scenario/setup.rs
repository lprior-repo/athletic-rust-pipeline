use super::*;
use census_domain::model::{AthleteCandidateId, SchoolId};
use census_domain::public_identity_history::PublicAthleteId;
use census_store::{JournalReadBudget, ReadBudget, RetainedCaptureRootSchemaV1};

pub(super) const RESULT_URL: &str =
    "https://mi.milesplit.com/meets/763888-mhsaa-lp-region-09-1-milford-2026/results/1304896/raw";
const SCHOOL_URL: &str = "https://mi.milesplit.com/teams/12439-detroit-catholic-central/roster";
const PROFILE_URL: &str = "https://mi.milesplit.com/athletes/7223777-zacchaeus-brocks";
const TARGET_URL: &str =
    "https://mi.milesplit.com/meets/760382-chsl-championships-2026/teams/12439";
const ARTICLE_URL: &str = "https://www.mhsaa.com/sports/boys-track-field/stories/performance-week-detroit-catholic-centrals-zacchaeus-brocks";
pub(super) const RAW: &[u8] = include_bytes!("../../tests/fixtures/milesplit-result.html");
pub(super) const ARTICLE: &[u8] = include_bytes!("../../../tests/fixtures/mhsaa-status.html");

pub(super) struct Input {
    pub school: SchoolId,
    pub school_packet: ProviderSchoolPacketV1,
    pub target: SourceTargetPacketV1,
    pub article: RetainedHtmlCaptureV1,
    pub context: String,
    pub public: PublicAthleteId,
    pub candidate: AthleteCandidateId,
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
        DateTime::parse_from_rfc3339(ended)?.to_utc(),
        body,
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: vec![
                ("User-Agent".into(), "OpenAI-research/0.1".into()),
                ("Accept".into(), "text/html".into()),
            ],
            response_header_bytes: headers.to_vec(),
            content_type: "text/html; charset=UTF-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: DateTime::parse_from_rfc3339(started)?.to_utc(),
        },
    )?)
}

fn school_packet() -> TestResult<ProviderSchoolPacketV1> {
    Ok(ProviderSchoolPacketV1::new(
        capture(
            "https://mi.milesplit.com/teams",
            include_bytes!("../../../provider_school/fixtures/mi-team-index.html"),
            include_bytes!("../../../provider_school/fixtures/mi-team-index.headers"),
            "2026-10-16T02:57:54Z",
            "2026-10-16T02:58:00Z",
        )?,
        capture(
            SCHOOL_URL,
            include_bytes!("../../../provider_school/fixtures/dcc-provider-roster.html"),
            include_bytes!("../../../provider_school/fixtures/dcc-provider-roster.headers"),
            "2026-10-16T00:49:38Z",
            "2026-10-16T00:49:43Z",
        )?,
    )?)
}

fn target_packet() -> TestResult<SourceTargetPacketV1> {
    Ok(SourceTargetPacketV1::new(
        capture(
            PROFILE_URL,
            include_bytes!("../../../provider_target/tests/fixtures/profile.html"),
            include_bytes!("../../../provider_target/tests/fixtures/profile.headers"),
            "2026-10-16T00:50:21Z",
            "2026-10-16T00:50:27Z",
        )?,
        capture(
            TARGET_URL,
            include_bytes!("../../../provider_target/tests/fixtures/linked-results.html"),
            include_bytes!("../../../provider_target/tests/fixtures/linked-results.headers"),
            "2026-10-16T00:49:13Z",
            "2026-10-16T00:49:19Z",
        )?,
    )?)
}

pub(super) fn locators() -> [SourceRowLocator; 3] {
    [
        SourceRowLocator {
            ordinal: 527,
            byte_offset: 80520,
            byte_length: 100,
        },
        SourceRowLocator {
            ordinal: 540,
            byte_offset: 81648,
            byte_length: 92,
        },
        SourceRowLocator {
            ordinal: 572,
            byte_offset: 84465,
            byte_length: 92,
        },
    ]
}

pub(super) fn install() -> TestResult<(tempfile::TempDir, Store, Input)> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("store");
    Store::create_public_identity_qualification_v3(&path)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&path)?.into_parts();
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    let school_packet = school_packet()?;
    let recording = Recording::new();
    let school = record_provider_school_intake(&store, &recording, &school_packet)?;
    crate::recording::prepare_recorded(recording.drain(), "school-correspondence-public-school")?
        .apply(&store)?;
    let target = target_packet()?;
    let budget = ReadBudget {
        rows: 65_536,
        bytes: 32 * 1024 * 1024,
        record_bytes: 2 * 1024 * 1024,
    };
    let ProviderTargetIntakeDispositionV1::ProjectedUnaccepted { candidate } =
        record_provider_target_intake(&store, &recording, &target, budget)?
    else {
        return Err("source target was not projected".into());
    };
    crate::recording::prepare_recorded(recording.drain(), "school-correspondence-public-target")?
        .apply(&store)?;
    let result = capture(
        RESULT_URL,
        RAW,
        include_bytes!("../../tests/fixtures/milesplit-result.headers"),
        "2026-10-16T00:06:38Z",
        "2026-10-16T00:06:57Z",
    )?;
    let document = read_retained_html_result_rows(&result, &locators())?;
    let context = document.context_reference().to_owned();
    record_retained_html_result_capture(&store, &recording, &document)?;
    crate::recording::prepare_recorded(recording.drain(), "school-correspondence-public-results")?
        .apply(&store)?;
    let birth = writer.establish_source_bound(
        &store,
        "school-correspondence-public-birth",
        &candidate.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    let public = birth.public_id().clone();
    let article = capture(
        ARTICLE_URL,
        ARTICLE,
        include_bytes!("../../../tests/fixtures/mhsaa-status.headers"),
        "2026-10-16T00:05:12Z",
        "2026-10-16T00:05:15Z",
    )?;
    drop(document);
    drop(writer);
    Ok((
        directory,
        store,
        Input {
            school,
            school_packet,
            target,
            article,
            context,
            public,
            candidate: candidate.cast(),
        },
    ))
}

pub(super) fn enroll(store: &Store) -> TestResult {
    let inputs = store.inspect_result_school_enrollment_v1(AttachmentReplayGrantV1::new())?;
    let prepared = store.prepare_result_school_enrollment_v1(
        inputs,
        census_store::ResultSchoolEnrollmentReviewV1 {
            operation: text("school-correspondence-public-enroll")?,
            actor: ResultSchoolReviewActorV1::LocalOperator,
            reviewed_at: at()?,
            rationale: text(
                "Explicit enrollment after source-bound public birth and original row attachment",
            )?,
        },
    )?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    check!(matches!(
        store.apply_result_school_enrollment_v1(prepared, &approved, &digest)?,
        Application::Written(_)
    ));
    Ok(())
}

pub(super) fn originals(store: &Store, input: &Input) -> TestResult<Vec<String>> {
    let snapshot = store.snapshot();
    let mut rows = Vec::new();
    for (schema, key) in [
        (
            RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1,
            input.school_packet.reference()?.as_str().to_owned(),
        ),
        (
            RetainedCaptureRootSchemaV1::ProviderTargetPacketV1,
            input.target.reference()?.as_str().to_owned(),
        ),
        (
            RetainedCaptureRootSchemaV1::HtmlResultCaptureV1,
            input.context.clone(),
        ),
    ] {
        let raw = snapshot
            .journal_payload_raw_bounded(
                schema.phase(),
                &key,
                JournalReadBudget::new(2 * 1024 * 1024)?,
            )?
            .ok_or("original root absent")?;
        rows.push(raw.get().to_owned());
    }
    let installed: CanonicalSchool = snapshot
        .get_merged(Table::Schools, input.school.as_str())?
        .ok_or("installed school ID absent")?;
    check!(eq; installed.id, input.school);
    rows.push(serde_json::to_string(
        &snapshot.scan::<CanonicalAthlete>(Table::Athletes)?,
    )?);
    let mut receipts = receipts(store)?;
    receipts.retain(|receipt| {
        !receipt
            .operation
            .starts_with("result_school_correspondence")
    });
    rows.push(serde_json::to_string(&receipts)?);
    Ok(rows)
}

pub(super) fn state(
    store: &Store,
) -> TestResult<(String, u64, u64, u64, Vec<census_store::Receipt>)> {
    let snapshot = store.snapshot();
    Ok((
        snapshot.tables_digest(&Table::ALL)?,
        store.evidence_generation(),
        snapshot.derived_generation(),
        snapshot.sequence(),
        receipts(store)?,
    ))
}

fn receipts(store: &Store) -> TestResult<Vec<census_store::Receipt>> {
    let mut rows = Vec::new();
    store.for_each_receipt(64, |receipt| {
        rows.push(receipt);
        Ok(())
    })?;
    rows.sort_by(|left, right| left.operation.cmp(&right.operation));
    Ok(rows)
}

pub(super) fn refresh_physical(store: &Store, input: &Input) -> TestResult {
    let mut refreshed: CanonicalSchool = store
        .snapshot()
        .get_merged(Table::Schools, input.school.as_str())?
        .ok_or("school before refresh")?;
    let original = serialized_bytes(&refreshed)?;
    refreshed
        .aliases
        .push("Later raw physical metadata alias".into());
    store.append(Table::Schools, &refreshed)?;
    let actual: CanonicalSchool = store
        .snapshot()
        .get_merged(Table::Schools, input.school.as_str())?
        .ok_or("school after refresh")?;
    check!(eq; actual.id, input.school);
    check!(actual
        .aliases
        .iter()
        .any(|alias| alias == "Later raw physical metadata alias"));
    check!(ne; serialized_bytes(&actual)?, original);
    Ok(())
}
