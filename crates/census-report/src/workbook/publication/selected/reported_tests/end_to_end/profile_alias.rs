use super::TestResult;
use crate::export::public_identity::tests::fixture::Fixture;
use crate::export::selected_test_fixture::consumer;
use crate::export::{ExportDataset, ExportJobRequest};
use census_crawl::participation_evidence::provider_target::profile_pr::SourceProfilePrV1;
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Confidence, Evidence, ProfileSelectionV1,
    ProvisionalInclusionV1, PublishedGraduation, QueryProfileV1, SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use census_store::{RetainedCaptureRootSchemaV1, Table};

#[test]
fn selected_job_retains_profile_prs_from_an_accepted_member_without_own_cohort() -> TestResult {
    let fixture = accepted_alias_fixture()?;
    let packet = crate::export::profile_pr_test_source::packet("9401", "2026-09-30T00:50:27Z")?;
    let expected = packet
        .read_profile_prs_if_published()?
        .ok_or("controlled published PR table")?
        .into_rows();
    check!(!expected.is_empty());
    fixture.store.journal_done(
        RetainedCaptureRootSchemaV1::ProviderTargetPacketV1.phase(),
        packet.reference()?.as_str(),
        &packet,
    )?;
    let (initial, objects) = consumer::capture(&fixture.store)?;
    let mut input = initial.selected_authority()?.profile().input().clone();
    input.include_provisional = ProvisionalInclusionV1::Include;
    drop(initial);
    objects.close()?;
    let profile = QueryProfileV1::new(input)?;
    fixture.store.register_profile(&profile)?;
    let selection = ProfileSelectionV1::new(fixture.store.store_identity()?, profile.id()?);
    let dataset = ExportDataset::for_job_request(
        &fixture.store,
        "accepted-alias-profile-prs",
        &ExportJobRequest::SelectedPublicationV1 { selection },
    )?;
    require_raw_counterexample(&dataset)?;
    require_selected_profile_prs(&dataset, &expected)?;
    let archive = tempfile::tempdir()?;
    let path = archive.path().join("accepted-alias.json");
    let digest = dataset.archive_digest()?;
    dataset.save_frozen(&path)?;
    let store_path = fixture.store.root().to_path_buf();
    drop(dataset);
    let Fixture {
        directory,
        store,
        writer,
        ..
    } = fixture;
    drop(writer);
    drop(store);
    directory.close()?;
    check!(!store_path.exists());
    let reopened = ExportDataset::reopen_frozen_with_digest(&path, &digest)?;
    require_raw_counterexample(&reopened)?;
    require_selected_profile_prs(&reopened, &expected)
}

fn accepted_alias_fixture() -> TestResult<Fixture> {
    let mut fixture = consumer::source_births_with(consumer::RosterCase::Missing)?;
    let [first, second, _] = &mut fixture.athletes;
    check!(ne; first.source.as_ref().ok_or("first source")?.id,
        second.source.as_ref().ok_or("second source")?.id);
    second.sports = vec![Sport::OutdoorTrack];
    let mut school = CanonicalSchool::new(
        UsJurisdiction::Alabama,
        "Controlled accepted alias school",
        "controlled accepted alias school",
        None,
    )
    .0;
    school.id = second.school.clone();
    fixture.store.append(Table::Schools, &school)?;
    let source = SourceRef::new(
        "controlled-distinct-alias-cohort",
        Some("https://controlled-alias.test/2027-result".to_owned()),
    );
    second.published_graduations.push(PublishedGraduation {
        grad_year: second.grad_year,
        source: source.clone(),
    });
    second.evidence.push(Evidence::parsed(source, "2026-09-30"));
    consumer::merge(&fixture)?;
    Ok(fixture)
}

fn profile_member(athlete: &CanonicalAthlete) -> bool {
    athlete.identities().any(|identity| {
        identity.namespace == SourceNamespace::MilesplitAthlete && identity.id == "9401"
    })
}

fn require_raw_counterexample(dataset: &ExportDataset) -> TestResult {
    let members = dataset
        .athletes
        .iter()
        .filter(|athlete| profile_member(athlete))
        .collect::<Vec<_>>();
    check!(eq; members.len(), 1);
    for member in &members {
        check!(member.observed_grades.is_empty());
        check!(member.published_graduations.is_empty());
        check!(eq; member.derived_cohort_confidence(), None);
    }
    let donor = dataset
        .athletes
        .iter()
        .find(|athlete| {
            athlete.identities().any(|identity| {
                identity.namespace == SourceNamespace::TfrrsAthlete && identity.id == "9402"
            }) && !profile_member(athlete)
        })
        .ok_or("distinct cohort-bearing alias")?;
    check!(eq; donor.derived_cohort_confidence(), Some(Confidence::HIGH));
    let identities = dataset.verified_public_identities()?;
    let owner = identities
        .public_id(donor.id.as_str())
        .ok_or("accepted alias owner")?;
    for member in members {
        check!(eq; identities.public_id(member.id.as_str()), Some(owner));
    }
    Ok(())
}

fn require_selected_profile_prs(
    dataset: &ExportDataset,
    expected: &[SourceProfilePrV1],
) -> TestResult {
    let derivation = crate::report::Derivation::for_publication(
        dataset,
        crate::report::Scope::AllSources,
        Some(2027),
        &[UsJurisdiction::Alabama, UsJurisdiction::Wisconsin],
    )?;
    check!(derivation.athletes().is_empty());
    let accepted = dataset.verified_public_identities()?;
    let members =
        crate::report::population::accepted_members(&dataset.athletes, accepted.aliases());
    let athlete = members
        .iter()
        .find(|athlete| profile_member(athlete))
        .ok_or("accepted identity with unioned cohort evidence")?;
    check!(eq; athlete.derived_cohort_confidence(), Some(Confidence::HIGH));
    let mut tables = dataset.reported_profile_prs_for(athlete);
    let table = tables.next().ok_or("accepted alias profile PR root")?;
    check!(tables.next().is_none());
    check!(eq; table.source.namespace, SourceNamespace::MilesplitAthlete);
    check!(eq; table.source.id, "9401");
    check!(eq; table.rows.as_slice(), expected);
    Ok(())
}
