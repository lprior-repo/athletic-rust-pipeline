use super::*;
use census_crawl::net::{
    assess_directory_roster_child, CapturedPublicDirectory, PublicResourceAcquisition,
};
use census_crawl::participation_evidence::provider_school::OwnedProviderSchoolPacketV1;
use census_domain::model::*;
use census_domain::UsJurisdiction;
use census_store::{
    AcquisitionReadBudget, CaptureWriterV1, PublicResourceCaptureOwnerV1, PublicResourceRootKindV1,
    PublicSchoolIntakePacketV1, Table,
};
use controlled_normalized_transport::{retain_owned, retain_root, ControlledNormalizedCapture};

pub(super) async fn mint(
    store: &Store,
    writer: &CaptureWriterV1,
    fetcher: &Fetcher,
) -> TestResult<(String, CanonicalSchool, CanonicalTeam)> {
    let owner = writer.public_resource_owner(store, UsJurisdiction::Michigan)?;
    roots(store, writer, &owner)?;
    let acquisition = PublicResourceAcquisition::new(store, writer, &owner);
    let directory = fetcher
        .get_public_resource_directory(&acquisition, &FetchOptions::default())
        .await?;
    let original_directory = directory
        .directory()
        .retained_observed()?
        .ok_or("owned directory")?
        .original()
        .clone();
    let roster_original = roster(store, &owner, &directory)?;
    let captured = fetcher
        .get_public_resource_roster(&acquisition, &directory, TEAM, &FetchOptions::default())
        .await?;
    check!(eq; captured.capture().body.as_slice(), ROSTER);
    check!(eq; captured.retained_observed()?.ok_or("owned roster")?.original(), &roster_original);
    let prepared =
        store.prepare_public_school_intake(&owner, &original_directory, &roster_original)?;
    let packet = prepared.packet().clone();
    let checked = OwnedProviderSchoolPacketV1::from_packet(packet.clone()).read()?;
    let applied = store.apply_public_school_intake(prepared)?;
    check!(applied.written());
    check!(eq; applied.receipt().operation,
        format!("public_resource_school_intake_v1:{}", packet.reference().as_str()));
    let (school, program) = parents(store, &packet, &checked)?;
    Ok((packet.reference().as_str().to_owned(), school, program))
}

fn roots(
    store: &Store,
    writer: &CaptureWriterV1,
    owner: &PublicResourceCaptureOwnerV1,
) -> TestResult<()> {
    for (kind, body, content_type, started_at, received_at, completed_at) in [
        (
            PublicResourceRootKindV1::Robots,
            b"User-agent: *\nAllow: /teams\n".as_slice(),
            "text/plain",
            "2026-10-02T12:00:00Z",
            "2026-10-02T12:00:01Z",
            "2026-10-02T12:00:02Z",
        ),
        (
            PublicResourceRootKindV1::MileSplitTeamDirectory,
            INDEX,
            "text/html; charset=utf-8",
            "2026-10-02T12:00:03Z",
            "2026-10-02T12:00:04Z",
            "2026-10-02T12:00:05Z",
        ),
    ] {
        retain_root(
            store,
            writer,
            owner,
            owner.for_root(store, kind)?,
            ControlledNormalizedCapture {
                body,
                content_type,
                started_at,
                received_at,
                completed_at,
            },
        )?;
    }
    Ok(())
}

fn roster(
    store: &Store,
    owner: &PublicResourceCaptureOwnerV1,
    directory: &CapturedPublicDirectory,
) -> TestResult<AcquisitionOccurrenceRefV1> {
    let parent = directory
        .directory()
        .retained_observed()?
        .ok_or("directory original")?;
    let evidence = owner.directory_evidence(
        store,
        parent.original(),
        &mut AcquisitionReadBudget::for_work(),
    )?;
    let assessed = assess_directory_roster_child(&evidence, directory.directory(), TEAM)?;
    let prepared = store.prepare_public_roster_child(&assessed)?;
    check!(eq; prepared.work().request().original_url(), ROSTER_URL);
    let capture = controlled_normalized_transport::prepare(
        prepared.work(),
        ControlledNormalizedCapture {
            body: ROSTER,
            content_type: "text/html; charset=utf-8",
            started_at: "2026-10-02T12:00:06Z",
            received_at: "2026-10-02T12:00:07Z",
            completed_at: "2026-10-02T12:00:08Z",
        },
    )?;
    let owned = store.register_public_roster_child(prepared)?;
    retain_owned(store, owned, capture)
}

fn parents(
    store: &Store,
    packet: &PublicSchoolIntakePacketV1,
    checked: &census_crawl::participation_evidence::provider_school::CheckedProviderSchoolPacketV1,
) -> TestResult<(CanonicalSchool, CanonicalTeam)> {
    let school = checked.school().clone();
    check!(eq; school.name.as_str(), "Detroit Catholic Central");
    check!(eq; checked.directory_proof().row().ordinal, 241);
    check!(eq; checked.directory_proof().row().byte_offset, 91718);
    check!(eq; checked.directory_proof().row().byte_length, 271);
    let day =
        chrono::DateTime::parse_from_rfc3339(packet.roster().response().observed_at())?.to_utc();
    let year = SchoolYear::from_date(&day.date_naive().to_string()).ok_or("capture school year")?;
    let program = CanonicalTeam {
        id: CanonicalTeam::mint(&school.id, Sport::IndoorTrack, Gender::Boys, year),
        school: school.id.clone(),
        sport: Sport::IndoorTrack,
        gender: Gender::Boys,
        school_year: year,
        level: None,
        source_identities: vec![SourceIdentity::new(SourceNamespace::MilesplitTeam, TEAM)],
        evidence: Vec::new(),
        retained_conflicts: Vec::new(),
    };
    store.append(Table::Schools, &school)?;
    store.append(Table::Teams, &program)?;
    store.append(
        Table::SourceObservations,
        &SourceObservation::School(checked.observation().clone()),
    )?;
    Ok((school, program))
}
