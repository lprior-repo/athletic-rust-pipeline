use super::*;
use census_crawl::net::PublicResourceAcquisition;
use census_crawl::participation_evidence::provider_school::OwnedProviderSchoolPacketV1;
use census_crawl::participation_evidence::provider_target::roster_child::read_owned_roster_profile_child_v1;
use census_domain::model::*;
use census_domain::roster_affiliation_authority::SourceRowLocator;
use census_domain::UsJurisdiction;
use census_report::export::{ExportDataset, ExportJobRequest};
use census_report::workbook;
use census_store::{PublicSchoolIntakePacketV1, Table};
use controlled_normalized_transport::ControlledNormalizedCapture;
use std::time::Duration;

mod publication;

#[tokio::test]
async fn owned_profile_source_pr_survives_selected_publication_replay_and_store_absence(
) -> TestResult {
    let fixture = fixture::Fixture::new().await?;
    let (athlete, profile, digest) = capture(&fixture).await?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&fixture.path)?.into_parts();
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    writer.establish_source_bound(
        &store,
        "controlled-owned-profile-reported-identity",
        &athlete.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    store.register_profile(&profile)?;
    let selection = ProfileSelectionV1::new(store.store_identity()?, profile.id()?);
    let request = ExportJobRequest::SelectedPublicationV1 { selection };
    let dataset = ExportDataset::for_job_request(&store, "owned-profile-publication", &request)?;
    check!(eq; store.walk_table(Table::Performances)?.rows, 0);
    drop(writer);
    publication::verify(fixture, store, dataset, &request, &digest)
}

async fn capture(
    fixture: &fixture::Fixture,
) -> TestResult<(CanonicalAthlete, QueryProfileV1, String)> {
    let (store, writer) = Store::open_current_qualification_v3(&fixture.path)?.into_parts();
    let packet = packet(&store, &fixture.reference)?;
    let owned = OwnedProviderSchoolPacketV1::from_packet(packet.clone());
    let checked = owned.read()?;
    let member = checked
        .roster_verdict()
        .roster()
        .ok_or("readable original roster")?
        .athletes
        .iter()
        .map(|member| member.athlete())
        .find(|athlete| athlete.grad_year == GradYear::CO2027)
        .ok_or("source Class of 2027 roster member")?;
    let athlete = athlete(&fixture.school.id, member)?;
    let owner = writer.public_resource_owner(&store, UsJurisdiction::Michigan)?;
    let profile = publication_profile(&owner.selected_query_profile(&store)?)?;
    let fetcher = Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::ZERO,
        Default::default(),
        Vec::new(),
    )?
    .with_origin_locks(
        fixture
            .path
            .parent()
            .ok_or("fixture parent")?
            .join("origin-locks"),
    )
    .with_offline(true);
    retain(&store, &owner, &packet, &member.athlete_id)?;
    let acquired = fetcher
        .get_public_resource_profile(
            &PublicResourceAcquisition::new(&store, &writer, &owner),
            &packet,
            &member.athlete_id,
            &FetchOptions::default(),
        )
        .await?;
    check!(acquired.capture().from_cache);
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    let retained = store.retain_public_profile_capture_v1(
        &owner,
        acquired
            .retained_observed()?
            .ok_or("owned profile")?
            .original(),
    )?;
    let digest = retained.reference()?.as_str().to_owned();
    store.append(Table::Athletes, &athlete)?;
    check!(eq; store.walk_table(Table::Performances)?.rows, 0);
    store.flush()?;
    Ok((athlete, profile, digest))
}

fn packet(store: &Store, reference: &str) -> TestResult<PublicSchoolIntakePacketV1> {
    let snapshot = store.snapshot();
    let retained = snapshot
        .read_retained_capture_root_v1(
            RetainedCaptureRootSchemaV1::ProviderSchoolObservedPacketV1,
            reference,
            &mut AttachmentReplayGrantV1::new(),
        )?
        .ok_or("closed original roster")?;
    Ok(PublicSchoolIntakePacketV1::decode(
        retained.payload().get().as_bytes(),
    )?)
}

fn publication_profile(original: &QueryProfileV1) -> TestResult<QueryProfileV1> {
    use census_domain::profile_execution::{
        selected_qualification_contract_ref_v3 as reference, ContractRoleV1 as Role,
    };
    let mut input = original.input().clone();
    input.include_provisional = ProvisionalInclusionV1::Include;
    input.source_plan = reference(Role::SourcePlan)?;
    input.completion_contract = reference(Role::Completion)?;
    input.policies = ProfilePoliciesV1 {
        cohort: reference(Role::Cohort)?,
        event: reference(Role::Event)?,
        identity: reference(Role::Identity)?,
        affiliation: reference(Role::Affiliation)?,
        contact: reference(Role::Contact)?,
        participation: reference(Role::Participation)?,
        publication: reference(Role::Publication)?,
    };
    Ok(QueryProfileV1::new(input)?)
}

fn athlete(
    school: &SchoolId,
    row: &census_crawl::milesplit::RosterAthlete,
) -> TestResult<CanonicalAthlete> {
    let source = SourceRef::new("milesplit", Some(row.profile_url.clone()));
    let mut athlete = CanonicalAthlete::new(
        school,
        &row.name,
        row.grad_year,
        row.gender,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, &row.athlete_id)
            .with_url(row.profile_url.clone()),
    )?;
    athlete.sports = vec![Sport::IndoorTrack];
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: row.grad_year,
        source: source.clone(),
    });
    athlete
        .evidence
        .push(Evidence::parsed(source, "2026-10-02"));
    Ok(athlete)
}

fn retain(
    store: &Store,
    owner: &census_store::PublicResourceCaptureOwnerV1,
    packet: &PublicSchoolIntakePacketV1,
    athlete: &str,
) -> TestResult {
    let owned_packet = OwnedProviderSchoolPacketV1::from_packet(packet.clone());
    let checked = read_owned_roster_profile_child_v1(&owned_packet, athlete)?;
    let evidence = owner.roster_profile_evidence(store, packet)?;
    let row = checked.row();
    let prepared = evidence.prepare(
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
    let capture = controlled_normalized_transport::prepare(
        prepared.work(),
        ControlledNormalizedCapture {
            body: body.as_bytes(),
            content_type: "text/html; charset=utf-8",
            started_at: "2026-10-02T12:00:09Z",
            received_at: "2026-10-02T12:00:10Z",
            completed_at: "2026-10-02T12:00:11Z",
        },
    )?;
    let owned = store.register_public_profile_child(prepared)?;
    controlled_normalized_transport::retain_owned(store, owned, capture)?;
    Ok(())
}
