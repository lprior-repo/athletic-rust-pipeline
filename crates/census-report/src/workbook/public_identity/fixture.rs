use crate::export::{
    CurrentAffiliationResolution, ExportDataset, ParticipationAssessment, ParticipationOutcome,
    VerifiedCurrentAffiliations, VerifiedParticipationView,
};
use crate::report::{Derivation, Scope};
use crate::workbook::Options;
use census_domain::model::*;
use census_domain::participation_candidates::ParticipationReasons;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::collections::BTreeMap;

pub(in crate::workbook) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(in crate::workbook) struct Fixture {
    pub dataset: ExportDataset,
    pub options: Options,
    _store: Store,
    _directory: tempfile::TempDir,
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
        let performance = append_performance(&store, &athlete)?;
        writer.establish_source_bound(
            &store,
            "positive-output-birth",
            &athlete.id.cast(),
            store.snapshot().revision(),
            &writer.head(&store)?,
        )?;
        let mut dataset = ExportDataset::load_public_identity_fixture(&store)?;
        bind(&mut dataset, &athlete, &performance)?;
        dataset.bind_public_identity_fixture_from_store(&store)?;
        Ok(Self {
            dataset,
            options: Options {
                grad_year: Some(2027),
                out: None,
                limit: None,
                scope: Scope::AllSources,
                jurisdictions: vec![UsJurisdiction::Michigan],
                school_year: SchoolYear::DEFAULT,
            },
            _store: store,
            _directory: directory,
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

fn append_performance(
    store: &Store,
    athlete: &CanonicalAthlete,
) -> TestResult<CanonicalPerformance> {
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
    store.append(Table::Teams, &team)?;
    store.append(Table::Meets, &meet)?;
    store.append(Table::Events, &event)?;
    store.append(Table::Performances, &performance)?;
    Ok(performance)
}

fn bind(
    dataset: &mut ExportDataset,
    athlete: &CanonicalAthlete,
    performance: &CanonicalPerformance,
) -> TestResult {
    let profile = ProfileId::parse(&"a".repeat(64))?;
    let cutoff = ProfileDate::parse("2026-10-01")?;
    let rule = ContractRefV1::new(1, 1, &"b".repeat(64))?;
    dataset.bind_selection_for_test(profile.clone(), cutoff.clone())?;
    dataset.bind_participation_rule_for_test(rule.clone())?;
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
