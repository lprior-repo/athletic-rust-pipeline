use super::*;
use census_domain::affiliation_candidates::{KnownBounds, ObjectRef};
use census_domain::model::{
    CanonicalTeam, CombinedComponentResult, CombinedDiscipline, CombinedEventCompletion,
    CombinedEventSpecification, CompetitionLevel, Evidence, HurdleSpecification, ImplementMass,
    ProfileDate, ProfileDateRangeV1, PublishedGraduation, SchoolId, SchoolYear, SourceEventLabel,
    SourceRef,
};
use census_domain::participation_candidates::ProofReference;
use std::num::NonZeroU64;
use std::path::Path;

pub(in crate::bests::tests) fn evidence(source: &str, url: Option<String>, date: &str) -> Evidence {
    Evidence::parsed(SourceRef::new(source, url), date)
}

pub(in crate::bests::tests) fn school_row(
    state: UsJurisdiction,
    name: &str,
    normalized: String,
    evidence: Evidence,
) -> (CanonicalSchool, SchoolId) {
    let (mut school, id) = CanonicalSchool::new(state, name, normalized, None);
    school.evidence = vec![evidence];
    (school, id)
}

pub(in crate::bests::tests) struct AthleteParams<'a> {
    pub school: &'a SchoolId,
    pub name: &'a str,
    pub grad_year: GradYear,
    pub gender: Gender,
    pub namespace: SourceNamespace,
    pub owner: &'a str,
    pub roster_source: &'a str,
    pub roster_url: Option<String>,
    pub profile_url: Option<&'a str>,
    pub sport: Sport,
    pub evidence: Evidence,
}

pub(in crate::bests::tests) fn athlete_row(
    params: AthleteParams<'_>,
) -> TestResult<CanonicalAthlete> {
    let mut row = CanonicalAthlete::new(
        params.school,
        params.name,
        params.grad_year,
        params.gender,
        SourceIdentity::new(params.namespace, params.owner),
    )?;
    row.published_graduations = vec![PublishedGraduation {
        grad_year: params.grad_year,
        source: SourceRef::new(params.roster_source, params.roster_url),
    }];
    if let Some(url) = params.profile_url {
        row.public_profile_urls = vec![url.into()];
    }
    row.evidence = vec![params.evidence];
    row.sports = vec![params.sport];
    Ok(row)
}

pub(in crate::bests::tests) struct TeamParams<'a> {
    pub school: &'a SchoolId,
    pub sport: Sport,
    pub gender: Gender,
    pub year: SchoolYear,
    pub evidence: Evidence,
}

pub(in crate::bests::tests) fn team_row(params: TeamParams<'_>) -> CanonicalTeam {
    CanonicalTeam {
        id: CanonicalTeam::mint(params.school, params.sport, params.gender, params.year),
        school: params.school.clone(),
        sport: params.sport,
        gender: params.gender,
        school_year: params.year,
        level: None,
        source_identities: Vec::new(),
        evidence: vec![params.evidence],
        retained_conflicts: Vec::new(),
    }
}

pub(in crate::bests::tests) struct MeetParams<'a> {
    pub state: Option<UsJurisdiction>,
    pub name: &'a str,
    pub date: &'a str,
    pub level: CompetitionLevel,
    pub sport: Sport,
    pub evidence: Evidence,
    pub id_seed: Option<&'a [&'a str]>,
}

pub(in crate::bests::tests) fn meet_row(params: MeetParams<'_>) -> TestResult<CanonicalMeet> {
    let mut meet =
        CanonicalMeet::new_checked(params.state, params.name, params.date, params.level)?;
    meet.sports = vec![params.sport];
    meet.evidence = vec![params.evidence];
    if let Some(seed) = params.id_seed {
        meet.id = Id::mint("meet", seed);
    }
    Ok(meet)
}

pub(in crate::bests::tests) struct EventParams {
    pub kind: EventKind,
    pub gender: Gender,
    pub specification: EventSpecification,
    pub owned: bool,
    pub labels: Vec<SourceEventLabel>,
    pub evidence: Vec<Evidence>,
}

pub(in crate::bests::tests) fn event_row(
    meet: &CanonicalMeet,
    params: EventParams,
) -> TestResult<CanonicalEvent> {
    let mut event = CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: params.kind,
            gender: params.gender,
            division: None,
            round: None,
        },
        params.specification,
    )?;
    if params.owned {
        event = crate::test_events::owned(event);
    }
    event.source_labels.extend(params.labels);
    event.evidence.extend(params.evidence);
    Ok(event)
}

pub(in crate::bests::tests) struct PerformanceParams<'a> {
    pub id_seed: &'a [&'a str],
    pub date: &'a str,
    pub mark: Mark,
    pub wind_mps: Option<f64>,
    pub place: Option<u16>,
    pub heat: Option<&'a str>,
    pub round: Option<&'a str>,
    pub timing: Option<TimingMethod>,
    pub source_key: String,
    pub evidence: Evidence,
}

pub(in crate::bests::tests) fn performance_row(
    athlete: &CanonicalAthlete,
    team: &CanonicalTeam,
    meet: &CanonicalMeet,
    event: &CanonicalEvent,
    params: PerformanceParams<'_>,
) -> CanonicalPerformance {
    CanonicalPerformance {
        id: Id::mint("perf", params.id_seed),
        athlete: athlete.id.clone(),
        team: team.id.clone(),
        event: event.id.clone(),
        meet: meet.id.clone(),
        date: params.date.into(),
        mark: params.mark,
        wind_mps: params.wind_mps,
        adjustment: None,
        place: params.place,
        heat: params.heat.map(Into::into),
        round: params.round.map(Into::into),
        timing: params.timing,
        observed_grade: None,
        evidence: vec![params.evidence],
        source_key: params.source_key,
        source_athlete: athlete.source.clone(),
        retained_conflicts: Vec::new(),
        combined_completion: None,
    }
}

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
        let roster = Some("https://example.test/ties/roster".to_owned());
        let (school, school_id) = school_row(
            UsJurisdiction::Wisconsin,
            "Tie fixture school",
            "tie fixture school".to_owned(),
            evidence("wiaa_results", roster.clone(), "2026-10-07"),
        );
        let athlete = athlete_row(AthleteParams {
            school: &school_id,
            name: "Tie fixture runner",
            grad_year: GradYear::CO2027,
            gender: Gender::Boys,
            namespace: SourceNamespace::MilesplitAthlete,
            owner: "ties-2027",
            roster_source: "wiaa_results",
            roster_url: roster,
            profile_url: None,
            sport: Sport::OutdoorTrack,
            evidence: evidence(
                "wiaa_results",
                Some("https://example.test/ties/roster".to_owned()),
                "2026-10-07",
            ),
        })?;
        let school_year = SchoolYear::new(2026).ok_or("tie fixture season")?;
        let team = team_row(TeamParams {
            school: &school_id,
            sport: Sport::OutdoorTrack,
            gender: Gender::Boys,
            year: school_year,
            evidence: evidence(
                "wiaa_results",
                Some("https://example.test/ties/roster".to_owned()),
                "2026-10-07",
            ),
        });
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
        let meet_name = format!("Tie fixture {label}");
        let meet = meet_row(MeetParams {
            state: Some(UsJurisdiction::Wisconsin),
            name: &meet_name,
            date: "2026-05-01",
            level: CompetitionLevel::Invitational,
            sport: Sport::OutdoorTrack,
            evidence: evidence.clone(),
            id_seed: None,
        })?;
        let event = event_row(
            &meet,
            EventParams {
                kind: case.kind.clone(),
                gender: Gender::Boys,
                specification,
                owned: false,
                labels: vec![SourceEventLabel {
                    source,
                    label: case.label.into(),
                }],
                evidence: vec![evidence.clone()],
            },
        )?;
        let performance = performance_row(
            &self.athlete,
            &self.team,
            &meet,
            &event,
            PerformanceParams {
                id_seed: &["tie-order", label],
                date,
                mark: case.tied.clone(),
                wind_mps: Some(1.0),
                place: Some(1),
                heat: Some("1"),
                round: Some("Final"),
                timing: Some(TimingMethod::Fat),
                source_key: label.into(),
                evidence,
            },
        );
        Observation {
            meet,
            event,
            performance,
        }
        .with_mark(case.tied.clone())
    }

    pub(in crate::bests::tests) fn store_with_selected_replay(
        &self,
        root: &Path,
    ) -> TestResult<(Store, census_store::PublicIdentityWriterV1)> {
        Store::create_public_identity_qualification_v3(root)?;
        let (store, writer) = Store::open_public_identity_qualification_v3(root)?.into_parts();
        writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
        let numeric = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "9401");
        let mut athlete = self.athlete.clone();
        athlete.source = Some(numeric);
        store.append(Table::Schools, &self.school)?;
        store.append(Table::Athletes, &athlete)?;
        store.append(Table::Teams, &self.team)?;
        Ok((store, writer))
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
        let mut before = ExportDataset::load(&store)?;
        before.lineage.policy_revision = 21;
        let winners = selected(&before)?;
        store.flush()?;
        drop(store);
        let reopened = Store::open(root)?;
        let mut after = ExportDataset::load(&reopened)?;
        after.lineage.policy_revision = 21;
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

pub(in crate::bests::tests) fn proof<'a>(
    performance: &'a CanonicalPerformance,
    name: &'a str,
) -> TestResult<ProofReference<'a>> {
    Ok(ProofReference {
        fact: ObjectRef::parse(name)?,
        capture: ObjectRef::parse("archived-source-bytes")?,
        interpretation: ObjectRef::parse("published-source-interpretation")?,
        locator: ObjectRef::parse(name)?,
        athlete: &performance.athlete,
        known: KnownBounds::new(
            ObjectRef::parse("retained-test-lineage")?,
            revision(3)?,
            revision(7)?,
        )?,
        acquired_at: "2026-10-07T00:00:00Z".parse()?,
    })
}

pub(in crate::bests::tests) fn revision(value: u64) -> TestResult<NonZeroU64> {
    NonZeroU64::new(value).ok_or_else(|| "nonzero revision".into())
}

pub(in crate::bests::tests) fn day(value: &str) -> TestResult<ProfileDate> {
    Ok(ProfileDate::parse(value)?)
}

pub(in crate::bests::tests) fn range(from: &str, through: &str) -> TestResult<ProfileDateRangeV1> {
    Ok(ProfileDateRangeV1::new(day(from)?, day(through)?)?)
}
