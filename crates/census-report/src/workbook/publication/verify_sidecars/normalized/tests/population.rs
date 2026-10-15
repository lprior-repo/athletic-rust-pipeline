use super::TestResult;
use census_domain::model::performance_comparison::{
    ComparisonPolicy, Measure, SurfaceClass, TimingClass, WindClass,
};
use census_domain::model::*;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

pub(super) struct Population {
    pub dataset: crate::export::ExportDataset,
    pub options: crate::workbook::Options,
    pub school: CanonicalSchool,
    pub athletes: [CanonicalAthlete; 2],
    pub performances: Vec<CanonicalPerformance>,
    pub meets: Vec<CanonicalMeet>,
    pub coaches: Vec<CanonicalCoach>,
    pub specification: EventSpecification,
    pub class: String,
    pub outside: AthleteId,
    pub excluded: PerformanceId,
    pub directory: tempfile::TempDir,
}

impl Population {
    pub fn new() -> TestResult<Self> {
        Self::build(None)
    }

    pub fn current() -> TestResult<Self> {
        let mut population = Self::new()?;
        let store = Store::open(population.directory.path().join("store"))?;
        population.dataset = crate::export::ExportDataset::load(&store)?;
        Ok(population)
    }

    pub fn with_unknown_historical_school() -> TestResult<(Self, SchoolId)> {
        let (_, historical) = CanonicalSchool::new(
            UsJurisdiction::NewJersey,
            "Unretained Prior High",
            "unretained prior high",
            Some("Jersey City"),
        );
        Ok((Self::build(Some(&historical))?, historical))
    }

    fn build(historical: Option<&SchoolId>) -> TestResult<Self> {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path().join("store"))?;
        let (school, id) = CanonicalSchool::new(
            UsJurisdiction::Michigan,
            "Readback High",
            "readback high",
            Some("Lansing"),
        );
        store.append(Table::Schools, &school)?;
        let source = SourceRef::new(
            "manual-readback",
            Some("https://example.test/result".into()),
        );
        let athletes = [
            athlete(&id, "Ada Runner", "ada", &source)?,
            athlete(&id, "No Mark", "none", &source)?,
        ];
        for athlete in &athletes {
            store.append(Table::Athletes, athlete)?;
        }
        let result_school = historical.map_or(&id, core::convert::identity);
        let team = CanonicalTeam {
            id: CanonicalTeam::mint(
                result_school,
                Sport::OutdoorTrack,
                Gender::Girls,
                SchoolYear::DEFAULT,
            ),
            school: result_school.clone(),
            sport: Sport::OutdoorTrack,
            gender: Gender::Girls,
            school_year: SchoolYear::DEFAULT,
            level: None,
            source_identities: Vec::new(),
            evidence: Vec::new(),
            retained_conflicts: Vec::new(),
        };
        store.append(Table::Teams, &team)?;
        let mut performances = Vec::new();
        let mut meets = Vec::new();
        for (index, (date, mark)) in [
            ("2026-05-01", "50.00"),
            ("2026-05-02", "50.00"),
            ("2026-05-03", "51.00"),
        ]
        .into_iter()
        .enumerate()
        {
            let (meet, event, performance) =
                observed(&athletes[0], &team, &source, index, date, mark)?;
            store.append(Table::Meets, &meet)?;
            store.append(Table::Events, &event)?;
            store.append(Table::Performances, &performance)?;
            performances.push(performance);
            meets.push(meet);
        }
        let excluded = excluded(&store, &athletes[1], &team, &source)?;
        let (outside_school, outside_id) = CanonicalSchool::new(
            UsJurisdiction::Wisconsin,
            "Outside High",
            "outside high",
            Some("Madison"),
        );
        store.append(Table::Schools, &outside_school)?;
        let outside = athlete(&outside_id, "Outside Runner", "outside", &source)?;
        store.append(Table::Athletes, &outside)?;
        let coaches = [CoachRole::HeadCoach, CoachRole::AssistantCoach]
            .into_iter()
            .map(|role| {
                let mut coach = CanonicalCoach::new(
                    &id,
                    role.stable_key(),
                    Some(Sport::OutdoorTrack),
                    Gender::Girls,
                    role,
                );
                coach.professional_email = Some("unadmitted@example.test".into());
                coach.phone = Some("555-0100".into());
                coach
            })
            .collect::<Vec<_>>();
        for coach in &coaches {
            store.append(Table::Coaches, coach)?;
        }
        let coverage = CoverageRow::new(CoverageScope::Jurisdiction, "MI")
            .with("registered", 7)
            .with("applicable", 7)
            .with("attempted", 6)
            .with("captured", 5)
            .with("parsed", 4)
            .with("accepted", 3)
            .with("withheld", 2);
        crate::report::CoverageReport::publish_coverage_rows(
            &store,
            std::slice::from_ref(&coverage),
        )?;
        let mut dataset = crate::export::ExportDataset::load(&store)?;
        dataset.lineage.policy_revision = 20;
        dataset.lineage.input_generation = serialized_digest(&(
            &dataset.lineage.store_identity,
            &dataset.lineage.source_digest,
            &dataset.lineage.input_digest,
            &dataset.lineage.generated_on,
            &dataset.lineage.export_job,
            dataset.lineage.schema_revision,
            20_u32,
        ))?;
        let options = crate::workbook::Options {
            grad_year: Some(2027),
            out: None,
            limit: None,
            scope: crate::report::Scope::AllSources,
            jurisdictions: vec![UsJurisdiction::Michigan],
            school_year: SchoolYear::DEFAULT,
        };
        let specification = EventSpecification {
            category: Some(CompetitionCategory::Girls),
            ..EventSpecification::default()
        };
        let class = census_domain::model::serialized_digest(&(
            EventKind::Track400m,
            SurfaceClass::Outdoor,
            WindClass::NotApplicable,
            TimingClass::Fat,
            Measure::Time,
            None::<EventId>,
            &specification,
            ComparisonPolicy::Standard,
            None::<()>,
            Gender::Girls,
        ))?;
        Ok(Self {
            dataset,
            options,
            school,
            athletes,
            performances,
            meets,
            coaches,
            specification,
            class,
            outside: outside.id,
            excluded,
            directory,
        })
    }

    pub fn inputs(&self) -> crate::report::ReportResult<super::super::super::input::Inputs<'_>> {
        super::super::super::input::Inputs::new(&self.dataset, &self.options)
    }
}

fn athlete(
    school: &SchoolId,
    name: &str,
    source_id: &str,
    source: &SourceRef,
) -> TestResult<CanonicalAthlete> {
    let mut athlete = CanonicalAthlete::new(
        school,
        name,
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, source_id),
    )?;
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: source.clone(),
    });
    athlete.sports.push(Sport::OutdoorTrack);
    Ok(athlete)
}

fn observed(
    athlete: &CanonicalAthlete,
    team: &CanonicalTeam,
    source: &SourceRef,
    index: usize,
    date: &str,
    mark: &str,
) -> TestResult<(CanonicalMeet, CanonicalEvent, CanonicalPerformance)> {
    let mut meet = CanonicalMeet::new_checked(
        Some(UsJurisdiction::Michigan),
        format!("Readback Meet {index}"),
        date,
        CompetitionLevel::Invitational,
    )?;
    meet.sports.push(Sport::OutdoorTrack);
    let event = crate::test_events::owned(CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: EventKind::Track400m,
            gender: Gender::Girls,
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
            date,
            source_key: &format!("readback-{index}"),
        },
        PerformanceResult {
            team: &team.id,
            mark: Mark::TimeSeconds(ExactSeconds::parse(mark)?),
            wind_mps: None,
            adjustment: None,
            place: Some(1),
        },
    )?;
    performance.timing = Some(TimingMethod::Fat);
    performance
        .evidence
        .push(Evidence::parsed(source.clone(), "2026-05-04"));
    Ok((meet, event, performance))
}

fn excluded(
    store: &Store,
    athlete: &CanonicalAthlete,
    team: &CanonicalTeam,
    source: &SourceRef,
) -> TestResult<PerformanceId> {
    let mut meet = CanonicalMeet::new_checked(
        Some(UsJurisdiction::Michigan),
        "Excluded Walk",
        "2026-05-01",
        CompetitionLevel::Invitational,
    )?;
    meet.sports.push(Sport::OutdoorTrack);
    let event = crate::test_events::owned(CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: EventKind::Unmapped {
                label: "Racewalk 100m".into(),
            },
            gender: Gender::Girls,
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
            source_key: "excluded-walk",
        },
        PerformanceResult {
            team: &team.id,
            mark: Mark::TimeSeconds(ExactSeconds::parse("40.00")?),
            wind_mps: None,
            adjustment: None,
            place: Some(1),
        },
    )?;
    performance.timing = Some(TimingMethod::Fat);
    performance
        .evidence
        .push(Evidence::parsed(source.clone(), "2026-05-04"));
    store.append(Table::Meets, &meet)?;
    store.append(Table::Events, &event)?;
    store.append(Table::Performances, &performance)?;
    Ok(performance.id)
}
