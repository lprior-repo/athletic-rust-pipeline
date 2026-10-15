use super::*;
use census_crawl::milesplit::{CaptureReadBudget, RetainedResultSession};
use census_domain::model::{CanonicalEvent, CanonicalMeet, CanonicalTeam};

#[tokio::test]
async fn grouped_capture_resolution_preserves_individual_assessments_for_reordered_input(
) -> TestResult {
    let fixture = Fixture::prepare(Some("High School")).await?;
    let snapshot = fixture.store.snapshot();
    let mut dataset = fixture.dataset(&snapshot)?;
    dataset.performances.reverse();
    let closure = closure::Closure {
        athletes: fixture.store.scan::<CanonicalAthlete>(Table::Athletes)?,
        performances: fixture
            .store
            .scan::<CanonicalPerformance>(Table::Performances)?,
        teams: fixture.store.scan::<CanonicalTeam>(Table::Teams)?,
        meets: fixture.store.scan::<CanonicalMeet>(Table::Meets)?,
        events: fixture.store.scan::<CanonicalEvent>(Table::Events)?,
    };
    let profile_id = fixture.profile.id()?;
    let query = CandidateQuery {
        profile: &fixture.profile,
        binding: CandidateBinding {
            profile: &profile_id,
            participation_rule: &fixture.profile.input().policies.participation,
            frozen: DeclaredFrozenView {
                lineage: object(&dataset.lineage.store_identity)?,
                revision: NonZeroU64::new(snapshot.sequence()).ok_or("snapshot revision")?,
                captured_at: DateTime::parse_from_rfc3339("2026-10-15T14:00:00Z")?
                    .with_timezone(&Utc),
                closure_object: object(&dataset.lineage.input_digest)?,
                family_inventory: object(&dataset.lineage.source_digest)?,
            },
        },
    };
    let affiliations =
        crate::export::qualification::current_affiliation::PinnedCurrentAffiliations::read(
            &dataset, &snapshot,
        )?;
    let mut resolver = resolve::Resolver {
        dataset: &dataset,
        identities: crate::export::public_identity::population::IdentityPopulation::of(&dataset)?,
        roster_context: affiliations.roster_context(),
        contexts: contexts::ResultContexts::default(),
        closure: &closure,
        sources: RetainedResultSession::new(&snapshot, CaptureReadBudget::new(32 * 1024 * 1024)?),
        budget: budget::Captures::default(),
    };
    let expected = dataset
        .performances
        .iter()
        .map(|performance| {
            resolver.sources.clear();
            Ok((
                performance.id.clone(),
                resolver.row(performance, query)?.into_parts(),
            ))
        })
        .collect::<TestResult<BTreeMap<_, _>>>()?;
    let actual = resolver
        .grouped_rows(query)?
        .into_iter()
        .map(|(id, row)| (id, row.into_parts()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(actual, expected);
    Ok(())
}
