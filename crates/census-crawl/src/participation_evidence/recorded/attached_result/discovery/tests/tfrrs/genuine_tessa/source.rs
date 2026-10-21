use super::*;
use crate::participation_evidence::provider_school::ProviderSchoolPacketRefV1;
use crate::participation_evidence::recorded::{
    record_independent_participation_packet, record_provider_school_intake,
};
use crate::participation_evidence::ParticipationEvidenceRefV1;
use census_domain::model::CanonicalSchool;

pub(super) struct Installed {
    pub directory: tempfile::TempDir,
    pub store: Store,
    pub school: CanonicalSchool,
    pub athlete: CanonicalAthlete,
    pub public: PublicAthleteId,
    pub school_reference: ProviderSchoolPacketRefV1,
    pub participation: ParticipationEvidenceRefV1,
    pub context: String,
    pub originals: Vec<(String, String, String)>,
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
    let school_packet = provider::packet()?;
    let school = school_packet.read()?.school().clone();
    let capture = common_fixture::capture(CheckedHtmlResultSourceV1::Tfrrs)?;
    let checked = read_retained_tfrrs_list_rows(&capture, &[locator()])?;
    let packet = roncalli_fixture::packet(roncalli_fixture::REVIEW, roncalli_fixture::SCHEDULE)?;
    let before = state(&store)?;
    let recording = Recording::new();
    check!(eq; record_provider_school_intake(&store, &recording, &school_packet)?, school.id);
    record_retained_tfrrs_list_capture(&store, &recording, &checked)?;
    record_independent_participation_packet(&store, &recording, &packet)?;
    check!(eq; state(&store)?, before);
    let recorded = recording.drain();
    let keys = recorded
        .journal
        .iter()
        .map(|row| (row.phase.clone(), row.key.clone()))
        .collect::<Vec<_>>();
    crate::recording::prepare_recorded(
        recorded,
        "genuine-tessa-research-originals-controlled-review",
    )?
    .apply(&store)?;
    let row = checked.row(0).ok_or("exact Tessa original result row")?;
    let participation = packet.read(store.store_identity()?, store.snapshot().sequence())?;
    check!(eq; row.athlete_id(), "8779607");
    check!(eq; row.team_name(), school.name);
    check!(eq; row.athlete_name(), participation.subject().name());
    let athlete = target(&school, &row, participation.cohort())?;
    store.append(Table::Athletes, &athlete)?;
    let birth = writer.establish_source_bound(
        &store,
        "genuine-tessa-native-tfrrs-birth-controlled-review",
        &athlete.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    check!(!birth.replayed());
    let public = birth.public_id().clone();
    drop(writer);
    let originals = keys
        .into_iter()
        .map(|(phase, key)| -> TestResult<_> {
            let raw = store
                .snapshot()
                .journal_payload_raw_bounded(
                    &phase,
                    &key,
                    JournalReadBudget::new(4 * 1024 * 1024)?,
                )?
                .ok_or("installed exact original payload")?;
            Ok((phase, key, raw.get().to_owned()))
        })
        .collect::<TestResult<Vec<_>>>()?;
    Ok(Installed {
        directory,
        store,
        school,
        athlete,
        public,
        school_reference: school_packet.reference()?,
        participation: packet.reference()?,
        context: checked.context_reference().into(),
        originals,
    })
}

fn target(
    school: &CanonicalSchool,
    row: &crate::participation_evidence::result_ownership::tfrrs::CheckedRetainedTfrrsRowV1<'_>,
    cohort: GradYear,
) -> TestResult<CanonicalAthlete> {
    let url = url::Url::parse("https://indiana.tfrrs.org")?.join(row.athlete_path())?;
    let mut target = CanonicalAthlete::new(
        &school.id,
        row.athlete_name(),
        cohort,
        row.gender(),
        SourceIdentity::new(SourceNamespace::TfrrsAthlete, row.athlete_id()).with_url(url.as_str()),
    )?;
    let mut evidence = Evidence::parsed(
        SourceRef::new("genuine-tfrrs-native-test-target/v1", Some(url.to_string())),
        "2026-10-21",
    );
    evidence.note = Some("Genuine native TFRRS row and exact Roncalli school; test-only source-bound identity review. Documentary participation is separately attached and exact-event-day scoped; no production human approval or tenure asserted.".into());
    target.evidence.push(evidence);
    Ok(target)
}

pub(super) fn originals(store: &Store, held: &[(String, String, String)]) -> TestResult {
    for (phase, key, expected) in held {
        let raw = store
            .snapshot()
            .journal_payload_raw_bounded(phase, key, JournalReadBudget::new(4 * 1024 * 1024)?)?
            .ok_or("retained Tessa original")?;
        check!(eq; raw.get(), expected);
    }
    Ok(())
}
