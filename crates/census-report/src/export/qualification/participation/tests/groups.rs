use super::*;
use census_crawl::milesplit::{CaptureReadBudget, RetainedResultSession};
use census_domain::model::{CanonicalEvent, CanonicalMeet, CanonicalTeam};
use std::collections::BTreeSet;

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
    let identities = crate::export::public_identity::population::IdentityPopulation::of(&dataset)?;
    let indexed = closure.indexed(&dataset, &identities);
    let mut resolver = resolve::Resolver {
        dataset: &dataset,
        identities,
        roster_context: affiliations.roster_context(),
        contexts: contexts::ResultContexts::default(),
        closure: indexed,
        sources: RetainedResultSession::new(&snapshot, CaptureReadBudget::new(32 * 1024 * 1024)?),
        budget: budget::Captures::default(),
        independent: None,
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
    check!(eq; actual, expected);
    Ok(())
}

#[tokio::test]
async fn unknown_candidates_do_not_group_with_raw_self_and_old_event_conflicts_remain() -> TestResult
{
    let fixture = Fixture::prepare(Some("High School")).await?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let affiliations =
        crate::export::qualification::current_affiliation::PinnedCurrentAffiliations::read(
            &dataset, &snapshot,
        )?;
    let identities = crate::export::public_identity::population::IdentityPopulation::of(&dataset)?;
    let mut closure = physical_closure(&snapshot)?;
    let mut unknown = closure.athletes.first().ok_or("athlete")?.clone();
    unknown.id = census_domain::model::AthleteId::mint("ath", &["absent-index-candidate"]);
    let unknown_evidence =
        Evidence::derived(SourceRef::id("absent-proof"), "2026-10-22", "synthetic");
    unknown.evidence.push(unknown_evidence.clone());
    unknown
        .retained_conflicts
        .push(census_domain::model::RetainedConflict::new(
            "synthetic",
            unknown.id.as_str(),
            "unknown candidate",
            "must stay outside known groups",
        ));
    let mut performance = fixture.performance.clone();
    performance.athlete = unknown.id.clone();
    closure.athletes.push(unknown);
    let contexts = contexts::ResultContexts::default();
    {
        let indexed = closure.indexed(&dataset, &identities);
        check!(eq; indexed.cohort(&identities, &performance), crate::export::participation_bridge::SourceCohortAgreement::Missing);
        let gaps = indexed.gate(
            &identities,
            &performance,
            &contexts,
            affiliations.roster_context(),
        )?;
        check!(gaps.contains(&ParticipationDiagnostic::IdentityUnaccepted));
        check!(gaps.contains(&ParticipationDiagnostic::SourceBindingMismatch));
        check!(!gaps.contains(&ParticipationDiagnostic::CanonicalConflict));
    }
    let mut historical = closure
        .events
        .iter()
        .find(|row| row.id == performance.event)
        .ok_or("event")?
        .clone();
    let event_evidence = Evidence::derived(
        SourceRef::id("historical-event-proof"),
        "2026-10-22",
        "synthetic",
    );
    historical.evidence.push(event_evidence.clone());
    historical
        .retained_conflicts
        .push(census_domain::model::RetainedConflict::new(
            "synthetic",
            historical.id.as_str(),
            "historical event",
            "retained contradiction",
        ));
    closure.events.insert(0, historical);
    closure.events.reverse();
    let indexed = closure.indexed(&dataset, &identities);
    let gaps = indexed.gate(
        &identities,
        &performance,
        &contexts,
        affiliations.roster_context(),
    )?;
    check!(gaps.contains(&ParticipationDiagnostic::CanonicalConflict));
    let mut references = BTreeSet::new();
    indexed.references(&identities, &performance, &mut references)?;
    check!(references.contains(&format!(
        "canonical_evidence:{}",
        serialized_digest(&event_evidence)?
    )));
    check!(!references.contains(&format!(
        "canonical_evidence:{}",
        serialized_digest(&unknown_evidence)?
    )));
    check!(eq;
        indexed.performances(&performance).count(),
        closure.performances.iter().filter(|row| row.id == performance.id).count()
    );
    Ok(())
}

#[tokio::test]
async fn reviewed_aliases_share_all_physical_cohort_evidence_in_existing_member_order() -> TestResult
{
    let performance_fixture = Fixture::prepare(Some("High School")).await?;
    let fixture = crate::export::public_identity::tests::fixture::source_births()?;
    crate::export::public_identity::tests::fixture::merge(&fixture)?;
    let snapshot = fixture.store.snapshot();
    let mut dataset =
        ExportDataset::load_public_identity_snapshot_fixture(&fixture.store, &snapshot)?;
    dataset.bind_selection_for_test(
        performance_fixture.profile.id()?,
        performance_fixture.profile.input().as_of.clone(),
    )?;
    dataset.bind_public_identity_fixture_from_snapshot(&snapshot)?;
    let identities = crate::export::public_identity::population::IdentityPopulation::of(&dataset)?;
    let mut closure = physical_closure(&snapshot)?;
    closure.athletes.reverse();
    let indexed = closure.indexed(&dataset, &identities);
    let [first, second, _] = &fixture.athletes;
    check!(identities.same_group(first.id.as_str(), second.id.as_str()));
    let mut performance = performance_fixture.performance.clone();
    performance.athlete = first.id.clone();
    let first_cohort = indexed.cohort(&identities, &performance);
    let mut first_references = BTreeSet::new();
    indexed.references(&identities, &performance, &mut first_references)?;
    performance.athlete = second.id.clone();
    check!(eq; indexed.cohort(&identities, &performance), first_cohort);
    let mut second_references = BTreeSet::new();
    indexed.references(&identities, &performance, &mut second_references)?;
    check!(eq; first_references, second_references);
    for athlete in &closure.athletes {
        if identities.same_group(athlete.id.as_str(), first.id.as_str()) {
            for evidence in &athlete.evidence {
                check!(first_references.contains(&format!(
                    "canonical_evidence:{}",
                    serialized_digest(evidence)?
                )));
            }
        }
    }
    Ok(())
}

fn physical_closure(snapshot: &census_store::StoreSnapshot<'_>) -> TestResult<closure::Closure> {
    Ok(closure::Closure {
        athletes: snapshot.scan(Table::Athletes)?,
        performances: snapshot.scan(Table::Performances)?,
        teams: snapshot.scan(Table::Teams)?,
        meets: snapshot.scan(Table::Meets)?,
        events: snapshot.scan(Table::Events)?,
    })
}
