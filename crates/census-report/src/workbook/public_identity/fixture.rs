use crate::export::{
    CurrentAffiliationResolution, ExportDataset, ParticipationAssessment, ParticipationOutcome,
    VerifiedCurrentAffiliations, VerifiedParticipationView,
};
use crate::report::{Derivation, Scope};
use crate::workbook::Options;
use census_domain::model::*;
use census_domain::participation_candidates::ParticipationReasons;
use census_domain::profile_execution::{
    selected_qualification_contract_ref_v3 as reference, ContractRoleV1 as Role,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::collections::BTreeMap;

pub(in crate::workbook) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(in crate::workbook) struct Fixture {
    pub dataset: ExportDataset,
    pub options: Options,
    _store: Store,
    _directory: tempfile::TempDir,
    _objects: tempfile::TempDir,
}

impl Fixture {
    pub fn new() -> TestResult<Self> {
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("identity");
        Store::create_public_identity_qualification_v3(&root)?;
        let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
        writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
        let (school, athlete) = school_and_athlete()?;
        store.append(Table::Schools, &school)?;
        store.append(Table::Athletes, &athlete)?;
        let (team, meet, event, performance) = build_performance(&athlete)?;
        writer.establish_source_bound(
            &store,
            "positive-output-birth",
            &athlete.id.cast(),
            store.snapshot().revision(),
            &writer.head(&store)?,
        )?;
        let profile = profile()?;
        store.register_profile(&profile)?;
        let selection = ProfileSelectionV1::new(store.store_identity()?, profile.id()?);
        let objects = tempfile::tempdir()?;
        let mut dataset = ExportDataset::from_selected_snapshot(
            &store,
            &store.snapshot(),
            &selection,
            objects.path(),
        )?;
        dataset.ensure_current_selected_write()?;
        if !dataset.teams.contains_key(&team.id) {
            dataset.teams.insert(team.id.clone(), team);
        }
        if !dataset.meets.iter().any(|row| row.id == meet.id) {
            dataset.meets.push(meet);
        }
        if !dataset.events.iter().any(|row| row.id == event.id) {
            dataset.events.push(event);
        }
        dataset.performances.push(performance.clone());
        dataset.bind_participation_rule_for_test(
            dataset
                .selected_participation_rule()
                .ok_or("captured rule")?
                .clone(),
        )?;
        dataset.bind_public_identity_fixture_from_store(&store)?;
        bind(&mut dataset, &athlete, &performance)?;
        Ok(Self {
            dataset,
            options: Options {
                address_corpus: None,
                grad_year: Some(2027),
                out: None,
                limit: None,
                scope: Scope::AllSources,
                jurisdictions: vec![UsJurisdiction::Michigan],
                school_year: SchoolYear::DEFAULT,
            },
            _store: store,
            _directory: directory,
            _objects: objects,
        })
    }

    pub fn derivation(&self) -> TestResult<Derivation<'_>> {
        Ok(Derivation::for_publication(
            &self.dataset,
            self.options.scope,
            self.options.grad_year,
            &self.options.jurisdictions,
        )?)
    }
}

fn build_performance(
    athlete: &CanonicalAthlete,
) -> TestResult<(
    CanonicalTeam,
    CanonicalMeet,
    CanonicalEvent,
    CanonicalPerformance,
)> {
    let team = CanonicalTeam {
        id: CanonicalTeam::mint(
            &athlete.school,
            Sport::OutdoorTrack,
            athlete.gender,
            SchoolYear::DEFAULT,
        ),
        school: athlete.school.clone(),
        sport: Sport::OutdoorTrack,
        gender: athlete.gender,
        school_year: SchoolYear::DEFAULT,
        level: None,
        source_identities: Vec::new(),
        evidence: Vec::new(),
        retained_conflicts: Vec::new(),
    };
    let mut meet = CanonicalMeet::new_checked(
        Some(UsJurisdiction::Michigan),
        "Synthetic public identity meet",
        "2026-05-01",
        CompetitionLevel::Invitational,
    )?;
    meet.sports.push(Sport::OutdoorTrack);
    let event = crate::test_events::owned(CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: EventKind::Track400m,
            gender: athlete.gender,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?);
    let mut performance = CanonicalPerformance::new(
        PerformanceIdentity {
            athlete: &athlete.id,
            event: &event.id,
            meet: &meet.id,
            date: "2026-05-01",
            source_key: "synthetic-positive-result",
        },
        PerformanceResult {
            team: &team.id,
            mark: Mark::TimeSeconds(ExactSeconds::parse("50.00")?),
            wind_mps: None,
            adjustment: None,
            place: Some(1),
        },
    )?;
    performance.timing = Some(TimingMethod::Fat);
    performance.source_athlete = athlete.source.clone();
    performance.evidence.push(Evidence::parsed(
        SourceRef::new(
            "synthetic-results",
            Some("https://synthetic-selected.test/results/1".into()),
        ),
        "2026-05-01",
    ));
    Ok((team, meet, event, performance))
}

fn bind(
    dataset: &mut ExportDataset,
    athlete: &CanonicalAthlete,
    performance: &CanonicalPerformance,
) -> TestResult {
    let profile = dataset
        .selected_profile_id()
        .ok_or("captured profile")?
        .clone();
    let cutoff = dataset.selected_as_of().ok_or("captured cutoff")?.clone();
    let rule = dataset
        .selected_participation_rule()
        .ok_or("captured rule")?
        .clone();
    let generation = dataset.lineage.input_generation.clone();
    let participation = VerifiedParticipationView::fixture(
        generation.clone(),
        profile.clone(),
        rule,
        BTreeMap::from([(
            performance.id.clone(),
            ParticipationAssessment::fixture(
                ParticipationOutcome::Eligible,
                ParticipationReasons::default(),
                vec!["synthetic-selected-result-proof".into()],
            ),
        )]),
    );
    let affiliations = VerifiedCurrentAffiliations::fixture(
        generation,
        profile,
        cutoff,
        BTreeMap::from([(
            athlete.id.clone(),
            CurrentAffiliationResolution::Resolved {
                school: athlete.school.clone(),
                evidence_refs: vec!["synthetic-selected-school-proof".into()],
            },
        )]),
    );
    dataset.bind_verified_views_for_test(participation, affiliations)?;
    Ok(())
}

fn profile() -> TestResult<QueryProfileV1> {
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![UsJurisdiction::Michigan],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::OutdoorTrack],
        competition_categories: vec![Gender::Girls],
        as_of: ProfileDate::parse("2026-10-01")?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::new(2026).ok_or("profile school year")?,
        source_plan: reference(Role::SourcePlan)?,
        acquisition_window: ProfileDateRangeV1::new(
            ProfileDate::parse("2026-01-01")?,
            ProfileDate::parse("2026-10-01")?,
        )?,
        completion_contract: reference(Role::Completion)?,
        policies: ProfilePoliciesV1 {
            cohort: reference(Role::Cohort)?,
            event: reference(Role::Event)?,
            identity: reference(Role::Identity)?,
            affiliation: reference(Role::Affiliation)?,
            contact: reference(Role::Contact)?,
            participation: reference(Role::Participation)?,
            publication: reference(Role::Publication)?,
        },
        include_provisional: ProvisionalInclusionV1::Exclude,
    })?)
}

fn school_and_athlete() -> TestResult<(CanonicalSchool, CanonicalAthlete)> {
    let (school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Michigan,
        "Synthetic output high",
        "synthetic output high",
        None,
    );
    let mut athlete = CanonicalAthlete::new(
        &school_id,
        "Synthetic public output runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "9801"),
    )?;
    let source = SourceRef::new(
        "synthetic-results",
        Some("https://synthetic-selected.test/athletes/9801".into()),
    );
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: source.clone(),
    });
    athlete
        .evidence
        .push(Evidence::parsed(source, "2026-05-01"));
    athlete.sports.push(Sport::OutdoorTrack);
    Ok((school, athlete))
}
