use super::*;
use census_crawl::participation_evidence::recorded::{
    record_independent_participation_packet, record_provider_roster_rows_v3,
    record_provider_school_intake,
};
use census_domain::model::AthleteCandidateId;

pub(super) struct Installed {
    pub directory: tempfile::TempDir,
    pub store: Store,
    pub identity_writer: census_store::PublicIdentityWriterV1,
    pub school: CanonicalSchool,
    pub program: CanonicalTeam,
    pub athlete: CanonicalAthlete,
    pub native_candidate: AthleteCandidateId,
    pub public: PublicAthleteId,
    pub school_reference: ProviderSchoolPacketRefV1,
    pub participation: ParticipationEvidenceRefV1,
    pub context: String,
    pub originals: Vec<(String, String, String)>,
}

pub(super) fn school_packet() -> TestResult<ProviderSchoolPacketV1> {
    captures::school_packet()
}
pub(super) fn locator() -> SourceRowLocator {
    SourceRowLocator {
        ordinal: 200,
        byte_offset: 313596,
        byte_length: 1349,
    }
}

pub(super) fn install() -> TestResult<Installed> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("store");
    Store::create_public_identity_qualification_v3(&path)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&path)?.into_parts();
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    let packet = school_packet()?;
    let school = packet.read()?.school().clone();
    let capture = captures::tfrrs()?;
    let checked = read_retained_tfrrs_list_rows(&capture, &[locator()])?;
    let participation = roncalli_fixture::packet()?;
    let originals = record(&store, &packet, &checked, &participation)?;
    let retained_status =
        census_crawl::participation_evidence::recorded::read_recorded_independent_participation(
            &store.snapshot(),
            &participation.reference()?,
            &mut AttachmentReplayGrantV1::new(),
        )?
        .ok_or("actual installed Roncalli original pair")?;
    let status = retained_status.checked();
    let row = checked.row(0).ok_or("genuine Tessa row")?;
    check!(eq; row.athlete_id(), "8779607");
    check!(eq; row.team_name(), school.name);
    check!(eq; row.athlete_name(), status.subject().name());
    let native_candidate = native_candidate(&school, &row, status.cohort());
    let program = program(&store, &school)?;
    let athlete = milesplit(&store, &school)?;
    let candidate = athlete.id.cast();
    check!(ne; athlete.id.as_str(), native_candidate.as_str());
    let index = store.athlete_identity_index()?;
    check!(index.isolated_source(&candidate));
    check!(!index.subjects().any(|subject| subject == &native_candidate));
    let birth = writer.establish_source_bound(
        &store,
        "genuine-tessa-ms-current-roster-source-birth-report-qualification",
        &candidate,
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    check!(!birth.replayed());
    Ok(Installed {
        directory,
        store,
        identity_writer: writer,
        school,
        program,
        athlete,
        native_candidate,
        public: birth.public_id().clone(),
        school_reference: packet.reference()?,
        participation: participation.reference()?,
        context: checked.context_reference().into(),
        originals,
    })
}

fn record(
    store: &Store,
    packet: &ProviderSchoolPacketV1,
    checked: &census_crawl::participation_evidence::result_ownership::tfrrs::CheckedRetainedTfrrsListV1<'_>,
    participation: &IndependentParticipationPacketV1,
) -> TestResult<Vec<(String, String, String)>> {
    let before = state(store)?;
    let recording = Recording::new();
    record_provider_school_intake(store, &recording, packet)?;
    record_provider_roster_rows_v3(store, &recording, packet)?;
    record_retained_tfrrs_list_capture(store, &recording, checked)?;
    record_independent_participation_packet(store, &recording, participation)?;
    check!(eq; state(store)?, before);
    let recorded = recording.drain();
    let keys = recorded
        .journal
        .iter()
        .map(|row| (row.phase.clone(), row.key.clone()))
        .collect::<Vec<_>>();
    census_crawl::recording::prepare_recorded(
        recorded,
        "genuine-tessa-research-originals-controlled-report-qualification",
    )?
    .apply(store)?;
    keys.into_iter()
        .map(|(phase, key)| -> TestResult<_> {
            let raw = store
                .snapshot()
                .journal_payload_raw_bounded(
                    &phase,
                    &key,
                    JournalReadBudget::new(4 * 1024 * 1024)?,
                )?
                .ok_or("genuine installed original")?;
            Ok((phase, key, raw.get().to_owned()))
        })
        .collect()
}

fn program(store: &Store, school: &CanonicalSchool) -> TestResult<CanonicalTeam> {
    let rows = store.snapshot().scan::<CanonicalTeam>(Table::Teams)?;
    let mut rows = rows.into_iter().filter(|row| {
        row.school == school.id
            && row.sport == Sport::IndoorTrack
            && row.gender == Gender::Girls
            && row.school_year.get() == 2026
    });
    let selected = rows
        .next()
        .ok_or("actual current Roncalli girls indoor program")?;
    check!(rows.next().is_none());
    Ok(selected)
}

fn milesplit(store: &Store, school: &CanonicalSchool) -> TestResult<CanonicalAthlete> {
    let rows = store.snapshot().scan::<CanonicalAthlete>(Table::Athletes)?;
    let mut rows = rows.iter().filter(|row| {
        row.school == school.id
            && row.identities().any(|identity| {
                identity.namespace == SourceNamespace::MilesplitAthlete && identity.id == "13658157"
            })
    });
    let selected = rows
        .next()
        .ok_or("actual native MS Tessa projected candidate")?;
    check!(rows.next().is_none());
    check!(eq; selected.canonical_name, "Tessa Mize");
    check!(eq; selected.grad_year, GradYear::CO2027);
    check!(eq; selected.gender, Gender::Girls);
    Ok(selected.clone())
}

pub(super) fn originals(store: &Store, held: &[(String, String, String)]) -> TestResult {
    for (phase, key, expected) in held {
        let raw = store
            .snapshot()
            .journal_payload_raw_bounded(phase, key, JournalReadBudget::new(4 * 1024 * 1024)?)?
            .ok_or("genuine original root retained")?;
        check!(eq; raw.get(), expected);
    }
    Ok(())
}

fn native_candidate(
    school: &CanonicalSchool,
    row: &census_crawl::participation_evidence::result_ownership::tfrrs::CheckedRetainedTfrrsRowV1<
        '_,
    >,
    cohort: GradYear,
) -> AthleteCandidateId {
    CanonicalAthlete::mint(
        &school.id,
        row.athlete_name(),
        cohort,
        row.gender(),
        &SourceIdentity::new(SourceNamespace::TfrrsAthlete, row.athlete_id()),
    )
    .cast()
}
