use super::super::*;
use census_domain::model::{
    CanonicalTeam, CombinedComponentResult, CombinedDiscipline, CombinedEventCompletion,
    CombinedEventSpecification, CompetitionLevel, Evidence, HurdleSpecification, ImplementMass,
    PublishedGraduation, SchoolYear, SourceEventLabel, SourceRef,
};
use std::path::Path;

pub(in crate::bests::tests) struct NumericCase {
    pub kind: EventKind,
    pub label: &'static str,
    pub tied: Mark,
    pub better: Mark,
    pub value: i64,
}

pub(in crate::bests::tests) fn numeric_cases() -> TestResult<[NumericCase; 3]> {
    Ok([
        NumericCase {
            kind: EventKind::Track400m,
            label: "400 Meter Dash",
            tied: Mark::TimeSeconds(ExactSeconds::parse("50.00")?),
            better: Mark::TimeSeconds(ExactSeconds::parse("49.99")?),
            value: 50_000_000_000,
        },
        NumericCase {
            kind: EventKind::LongJump,
            label: "Long Jump",
            tied: Mark::DistanceMetres(CentiMetres::new(700)),
            better: Mark::DistanceMetres(CentiMetres::new(701)),
            value: 7_000_000,
        },
        NumericCase {
            kind: EventKind::Decathlon,
            label: "Decathlon",
            tied: Mark::Points(CentiPoints::new(500000)),
            better: Mark::Points(CentiPoints::new(500001)),
            value: 500000,
        },
    ])
}

pub(in crate::bests::tests) struct TieFixture {
    school: CanonicalSchool,
    athlete: CanonicalAthlete,
    team: CanonicalTeam,
}

#[derive(Clone)]
pub(in crate::bests::tests) struct Observation {
    pub meet: CanonicalMeet,
    pub event: CanonicalEvent,
    pub performance: CanonicalPerformance,
}

pub(in crate::bests::tests) fn completion_with_total(
    specification: CombinedEventSpecification,
    total: CentiPoints,
) -> TestResult<CombinedEventCompletion> {
    let count = i32::try_from(specification.lineup().len())?;
    let share = total.value().checked_div(count).ok_or("component share")?;
    let remainder = total
        .value()
        .checked_rem(count)
        .ok_or("component remainder")?;
    let components = specification
        .lineup()
        .iter()
        .enumerate()
        .map(|(index, _)| -> TestResult<CombinedComponentResult> {
            let points = if index == 0 {
                share.checked_add(remainder).ok_or("component points")?
            } else {
                share
            };
            Ok(CombinedComponentResult::Completed(CentiPoints::new(points)))
        })
        .collect::<TestResult<Vec<_>>>()?;
    Ok(CombinedEventCompletion::new(specification, components)?)
}

fn decathlon_specification() -> TestResult<EventSpecification> {
    let lineup = [
        (EventKind::Track100m, None, None),
        (EventKind::LongJump, None, None),
        (
            EventKind::ShotPut,
            None,
            Some(ImplementMass::try_from(6_000_000_000)?),
        ),
        (EventKind::HighJump, None, None),
        (EventKind::Track400m, None, None),
        (
            EventKind::Track110mHurdles,
            Some(HurdleSpecification::new(990_600, Some(9_140_000))?),
            None,
        ),
        (
            EventKind::Discus,
            None,
            Some(ImplementMass::try_from(1_750_000_000)?),
        ),
        (EventKind::PoleVault, None, None),
        (
            EventKind::Javelin,
            None,
            Some(ImplementMass::try_from(800_000_000)?),
        ),
        (EventKind::Track1500m, None, None),
    ]
    .into_iter()
    .map(|(kind, hurdles, implement)| CombinedDiscipline::new(kind, hurdles, implement))
    .collect::<Result<Vec<_>, _>>()?;
    Ok(EventSpecification {
        combined_event: Some(CombinedEventSpecification::new(
            EventKind::Decathlon,
            lineup,
            "Synthetic tie fixture scoring".into(),
            1,
        )?),
        ..EventSpecification::default()
    })
}

impl Observation {
    pub fn with_mark(mut self, mark: Mark) -> TestResult<Self> {
        if let (Some(specification), Mark::Points(total)) =
            (&self.event.specification.combined_event, &mark)
        {
            self.performance.combined_completion =
                Some(completion_with_total(specification.clone(), *total)?);
        }
        self.performance.mark = mark;
        Ok(self)
    }
}

impl TieFixture {
    pub fn new() -> TestResult<Self> {
        let (mut school, school_id) = CanonicalSchool::new(
            UsJurisdiction::Wisconsin,
            "Tie fixture school",
            "tie fixture school",
            None,
        );
        let source = SourceRef::new(
            "wiaa_results",
            Some("https://example.test/ties/roster".into()),
        );
        let evidence = Evidence::parsed(source.clone(), "2026-10-07");
        school.evidence = vec![evidence.clone()];
        let mut athlete = CanonicalAthlete::new(
            &school_id,
            "Tie fixture runner",
            GradYear::CO2027,
            Gender::Boys,
            SourceIdentity::new(SourceNamespace::MilesplitAthlete, "ties-2027"),
        )?;
        athlete.published_graduations = vec![PublishedGraduation {
            grad_year: GradYear::CO2027,
            source,
        }];
        athlete.evidence = vec![evidence.clone()];
        athlete.sports = vec![Sport::OutdoorTrack];
        let school_year = SchoolYear::new(2026).ok_or("tie fixture season")?;
        let team = CanonicalTeam {
            id: CanonicalTeam::mint(&school_id, Sport::OutdoorTrack, Gender::Boys, school_year),
            school: school_id,
            sport: Sport::OutdoorTrack,
            gender: Gender::Boys,
            school_year,
            level: None,
            source_identities: Vec::new(),
            evidence: vec![evidence],
            retained_conflicts: Vec::new(),
        };
        Ok(Self {
            school,
            athlete,
            team,
        })
    }

    pub fn report(&self, case: &NumericCase, label: &str, date: &str) -> TestResult<Observation> {
        let specification = match case.kind {
            EventKind::Decathlon => decathlon_specification()?,
            _ => EventSpecification::default(),
        };
        self.report_with_specification(case, label, date, specification)
    }

    pub fn report_with_specification(
        &self,
        case: &NumericCase,
        label: &str,
        date: &str,
        specification: EventSpecification,
    ) -> TestResult<Observation> {
        let source = SourceRef::new(
            "wiaa_results",
            Some(format!("https://example.test/ties/{label}")),
        );
        let evidence = Evidence::parsed(source.clone(), "2026-10-07");
        let mut meet = CanonicalMeet::new_checked(
            Some(UsJurisdiction::Wisconsin),
            format!("Tie fixture {label}"),
            "2026-05-01",
            CompetitionLevel::Invitational,
        )?;
        meet.sports = vec![Sport::OutdoorTrack];
        meet.evidence = vec![evidence.clone()];
        let mut event = CanonicalEvent::new(
            EventIdentity {
                meet: &meet.id,
                kind: case.kind.clone(),
                gender: Gender::Boys,
                division: None,
                round: None,
            },
            specification,
        )?;
        event.source_labels = vec![SourceEventLabel {
            source,
            label: case.label.into(),
        }];
        event.evidence = vec![evidence.clone()];
        let performance = CanonicalPerformance {
            id: Id::mint("perf", &["tie-order", label]),
            athlete: self.athlete.id.clone(),
            team: self.team.id.clone(),
            event: event.id.clone(),
            meet: meet.id.clone(),
            date: date.into(),
            mark: case.tied.clone(),
            wind_mps: Some(1.0),
            adjustment: None,
            place: Some(1),
            heat: Some("1".into()),
            round: Some("Final".into()),
            timing: Some(TimingMethod::Fat),
            observed_grade: None,
            evidence: vec![evidence],
            source_key: label.into(),
            source_athlete: self.athlete.source.clone(),
            retained_conflicts: Vec::new(),
            combined_completion: None,
        };
        Observation {
            meet,
            event,
            performance,
        }
        .with_mark(case.tied.clone())
    }

    pub fn store_with_replay(
        &self,
        root: &Path,
        observations: &[Observation],
    ) -> TestResult<Store> {
        let store = Store::open(root)?;
        store.append(Table::Schools, &self.school)?;
        store.append(Table::Athletes, &self.athlete)?;
        store.append(Table::Teams, &self.team)?;
        let first =
            observation_batch(&store, observations)?.commit_once("tie-order", "tie-digest")?;
        let expected = u64::try_from(observations.len())?
            .checked_mul(3)
            .ok_or("fixture count")?;
        check!(eq; first.appended(), expected);
        let before = store.stats()?;
        let replay =
            observation_batch(&store, observations)?.commit_once("tie-order", "tie-digest")?;
        check!(eq; replay.appended(), 0);
        let after = store.stats()?;
        check!(eq; (after.tables, after.appended, after.observations), (before.tables, before.appended, before.observations));
        retained_observations(&store, observations)?;
        let before = ExportDataset::load(&store)?;
        let winners = selected(&before)?;
        store.flush()?;
        drop(store);
        let reopened = Store::open(root)?;
        let after = ExportDataset::load(&reopened)?;
        check!(eq; after.performances, before.performances);
        check!(eq; selected(&after)?, winners);
        retained_observations(&reopened, observations)?;
        Ok(reopened)
    }
}

fn observation_batch<'a>(
    store: &'a Store,
    observations: &[Observation],
) -> TestResult<census_store::StoreBatch<'a>> {
    let mut batch = store.write_batch();
    observations
        .iter()
        .try_for_each(|observation| -> TestResult {
            batch.append_many(Table::Meets, std::slice::from_ref(&observation.meet))?;
            batch.append_many(Table::Events, std::slice::from_ref(&observation.event))?;
            batch.append_many(
                Table::Performances,
                std::slice::from_ref(&observation.performance),
            )?;
            Ok(())
        })?;
    Ok(batch)
}

fn retained_observations(store: &Store, observations: &[Observation]) -> TestResult {
    let mut physical = Vec::new();
    store.snapshot().for_each_observation(
        Table::Performances,
        |performance: CanonicalPerformance| {
            physical.push(performance);
            Ok(())
        },
    )?;
    check!(eq; physical.len(), observations.len());
    observations
        .iter()
        .try_for_each(|observation| -> TestResult {
            check!(physical.contains(&observation.performance));
            Ok(())
        })
}

pub(in crate::bests::tests) fn orders(
    observations: &[Observation],
) -> TestResult<Vec<Vec<Observation>>> {
    if observations.is_empty() || observations.len() > 4 {
        return Err("tie permutations require one through four observations".into());
    }
    let indices = (0..observations.len()).fold(vec![Vec::new()], |orders, _| {
        orders
            .into_iter()
            .flat_map(|prefix| {
                (0..observations.len()).filter_map(move |index| {
                    if prefix.contains(&index) {
                        return None;
                    }
                    Some(
                        prefix
                            .iter()
                            .copied()
                            .chain(std::iter::once(index))
                            .collect::<Vec<_>>(),
                    )
                })
            })
            .collect::<Vec<_>>()
    });
    indices
        .into_iter()
        .map(|order| {
            order
                .into_iter()
                .map(|index| {
                    observations
                        .get(index)
                        .cloned()
                        .ok_or_else(|| "invalid permutation index".into())
                })
                .collect()
        })
        .collect()
}
