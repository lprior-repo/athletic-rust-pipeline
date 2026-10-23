use super::super::controlled_capture::controlled_normalized_transport as controlled;
use super::{public_resources_fixture, Fixture, TestResult};
use census_crawl::net::{assess_directory_roster_child, FetchOptions, PublicResourceAcquisition};
use census_crawl::participation_evidence::provider_school::OwnedProviderSchoolPacketV1;
use census_crawl::participation_evidence::provider_target::roster_child::read_owned_roster_profile_child_v1;
use census_domain::model::GradYear;
use census_domain::roster_affiliation_authority::SourceRowLocator;
use census_domain::UsJurisdiction;
use census_store::{AcquisitionReadBudget, PublicSchoolIntakePacketV1};
use std::sync::Arc;

const INDEX: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../census-crawl/src/participation_evidence/provider_school/fixtures/mi-team-index.html"
));
const ROSTER: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../census-crawl/src/participation_evidence/provider_school/fixtures/dcc-provider-roster.html"
));

pub(super) struct Expected {
    pub(super) packet: String,
    pub(super) members: Vec<(String, usize)>,
    pub(super) source_gaps: u64,
}

pub(super) async fn install(fixture: &Fixture) -> TestResult<Expected> {
    public_resources_fixture::declare(
        &fixture.store,
        &fixture.writer,
        &fixture.request,
        &fixture.continued,
    )?;
    super::super::controlled_capture::install(
        &fixture.store,
        &fixture.writer,
        UsJurisdiction::Michigan,
        INDEX,
    )?;
    let packet = mint(fixture).await?;
    let checked = OwnedProviderSchoolPacketV1::from_packet(packet.clone()).read()?;
    let verdict = checked.roster_verdict();
    check!(matches!(
        verdict,
        census_crawl::milesplit::RosterVerdict::Partial { .. }
    ));
    check!(eq; verdict.rejections().len(), 18);
    check!(verdict.unfinished().is_none());
    check!(
        verdict
            .rejections()
            .iter()
            .all(|row| row.kind
                == census_crawl::milesplit::RosterRejectionKind::InvalidGraduationYear)
    );
    let roster = checked
        .roster_verdict()
        .roster()
        .ok_or("closed DCC roster")?;
    check!(eq; roster.athletes.len(), 146);
    let mut members = Vec::new();
    for (index, member) in roster.athletes.iter().enumerate() {
        if member.athlete().grad_year == GradYear::CO2027 {
            members.push((
                member.athlete().athlete_id.clone(),
                index.checked_add(1).ok_or("member ordinal overflow")?,
            ));
            retain_profile(fixture, &packet, &member.athlete().athlete_id)?;
        }
    }
    check!(eq; members.len(), 45);
    *fixture.owner.fetcher.lock().await = Some((Vec::new(), 1, Arc::clone(&fixture.continued)));
    check!(eq; fixture.continued.stats().await.physical_requests(), 0);
    Ok(Expected {
        packet: packet.reference().as_str().to_owned(),
        members,
        source_gaps: u64::try_from(verdict.rejections().len())?,
    })
}

async fn mint(fixture: &Fixture) -> TestResult<PublicSchoolIntakePacketV1> {
    let store = &fixture.store;
    let writer = &fixture.writer;
    let owner = writer.public_resource_owner(store, UsJurisdiction::Michigan)?;
    let acquisition = PublicResourceAcquisition::new(store, writer, &owner);
    let directory = fixture
        .continued
        .get_public_resource_directory(&acquisition, &FetchOptions::default())
        .await?;
    let original = directory
        .directory()
        .retained_observed()?
        .ok_or("owned directory")?;
    let evidence = owner.directory_evidence(
        store,
        original.original(),
        &mut AcquisitionReadBudget::for_work(),
    )?;
    let assessed = assess_directory_roster_child(&evidence, directory.directory(), "12439")?;
    let child = store.prepare_public_roster_child(&assessed)?;
    let capture = controlled::prepare(
        child.work(),
        controlled::ControlledNormalizedCapture {
            body: ROSTER,
            content_type: "text/html; charset=utf-8",
            started_at: "2026-10-02T12:00:06Z",
            received_at: "2026-10-02T12:00:07Z",
            completed_at: "2026-10-02T12:00:08Z",
        },
    )
    .map_err(boxed)?;
    controlled::retain_owned(store, store.register_public_roster_child(child)?, capture)
        .map_err(boxed)?;
    let roster = fixture
        .continued
        .get_public_resource_roster(&acquisition, &directory, "12439", &FetchOptions::default())
        .await?;
    let retained = roster.retained_observed()?.ok_or("owned roster")?;
    let prepared =
        store.prepare_public_school_intake(&owner, original.original(), retained.original())?;
    let packet = prepared.packet().clone();
    let applied = store.apply_public_school_intake(prepared)?;
    check!(applied.written());
    Ok(packet)
}

fn retain_profile(
    fixture: &Fixture,
    packet: &PublicSchoolIntakePacketV1,
    athlete: &str,
) -> TestResult {
    let store = &fixture.store;
    let owner = fixture
        .writer
        .public_resource_owner(store, UsJurisdiction::Michigan)?;
    let owned = OwnedProviderSchoolPacketV1::from_packet(packet.clone());
    let checked = read_owned_roster_profile_child_v1(&owned, athlete)?;
    let row = checked.row();
    let prepared = owner.roster_profile_evidence(store, packet)?.prepare(
        athlete,
        &checked.athlete().profile_url,
        SourceRowLocator::new(row.ordinal, row.byte_offset, row.byte_length),
        checked.row_digest(),
        checked.kernel_digest(),
    )?;
    let body = format!(
        "<!doctype html><link rel=\"canonical\" href=\"{}\"><table id=\"prTable\" class=\"pr-table\"><tr data-pr-mode=\"indoor\" data-pr-level=\"hs\"><td class=\"pr-td-event\">1600 Meter Run</td><td class=\"pr-td-mark\">4:30.00</td><td class=\"pr-td-date\">2025-03-01</td></tr></table>",
        checked.athlete().profile_url,
    );
    let capture = controlled::prepare(
        prepared.work(),
        controlled::ControlledNormalizedCapture {
            body: body.as_bytes(),
            content_type: "text/html; charset=utf-8",
            started_at: "2026-10-02T12:00:09Z",
            received_at: "2026-10-02T12:00:10Z",
            completed_at: "2026-10-02T12:00:11Z",
        },
    )
    .map_err(boxed)?;
    controlled::retain_owned(
        store,
        store.register_public_profile_child(prepared)?,
        capture,
    )
    .map_err(boxed)?;
    Ok(())
}

fn boxed(error: Box<dyn std::error::Error + Send + Sync>) -> Box<dyn std::error::Error> {
    error
}
