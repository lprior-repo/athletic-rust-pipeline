use crate::export::ExportDataset;
use census_domain::model::*;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) fn dataset() -> TestResult<ExportDataset> {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let (school, school_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "CEN school", "cen school", None);
    let mut athlete = CanonicalAthlete::new(
        &school_id,
        "CEN runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "1001"),
    );
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: SourceRef::new(
            "published_roster",
            Some("https://example.test/roster".into()),
        ),
    });
    store.append(Table::Schools, &school)?;
    store.append(Table::Athletes, &athlete)?;
    Ok(ExportDataset::load(&store)?)
}

pub(super) fn meet(name: &str, sport: Sport, date: &str) -> CanonicalMeet {
    CanonicalMeet {
        id: Id::mint("meet", &[name]),
        name: name.into(),
        normalized_name: name.into(),
        date: date.into(),
        end_date: None,
        location: None,
        state: Some(UsJurisdiction::Wisconsin),
        level: CompetitionLevel::Unknown,
        sports: vec![sport],
        source_identities: Vec::new(),
        source_urls: Vec::new(),
        evidence: Vec::new(),
        retained_conflicts: Vec::new(),
    }
}

pub(super) fn event(meet: &CanonicalMeet, label: &str) -> TestResult<CanonicalEvent> {
    let kind = EventKind::from_source_label(label);
    let specification = EventSpecification::from_published_label(label, &kind)?;
    event_with_specification(meet, label, Gender::Boys, specification)
}

pub(super) fn event_with_specification(
    meet: &CanonicalMeet,
    label: &str,
    gender: Gender,
    specification: EventSpecification,
) -> TestResult<CanonicalEvent> {
    let url = format!("https://example.test/event/{}", meet.id);
    let body =
        serde_json::json!({"_source": {"i": 1, "mi": 1, "n": label, "un": label, "ab": label,
        "gl": gender.stable_key(), "xc": meet.sports.contains(&Sport::CrossCountry), "r": []}})
        .to_string();
    let doc = census_crawl::athleticlive::parse_event_document(&url, &body)?;
    let mut event = CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: doc.kind(),
            gender,
            division: None,
            round: None,
        },
        specification,
    )?;
    let source = SourceRef::new("athleticlive_results", Some(url));
    event.source_labels.push(SourceEventLabel {
        source: source.clone(),
        label: doc.label().ok_or("event label")?.into(),
    });
    event.evidence.push(Evidence::parsed(source, &meet.date));
    Ok(event)
}

pub(super) fn add(
    dataset: &mut ExportDataset,
    meet: CanonicalMeet,
    event: CanonicalEvent,
    time: &str,
) -> TestResult {
    let athlete = dataset.athletes.first().ok_or("athlete")?;
    let mark = if event.kind.is_field() {
        Mark::DistanceMetres(CentiMetres::new(time.parse()?))
    } else {
        Mark::TimeSeconds(ExactSeconds::parse(time)?)
    };
    let mut performance = CanonicalPerformance::new(
        census_domain::model::PerformanceIdentity {
            athlete: &athlete.id,
            event: &event.id,
            meet: &meet.id,
            date: &meet.date,
            source_key: "cen",
        },
        census_domain::model::PerformanceResult {
            team: &Id::mint("team", &["cen"]),
            mark,
            wind_mps: None,
            place: None,
        },
    )?;
    performance.source_athlete = athlete.source.clone();
    performance.timing = Some(TimingMethod::Fat);
    dataset.performances.push(performance);
    dataset.events.push(event);
    dataset.meets.push(meet);
    Ok(())
}

pub(super) fn selections(dataset: &ExportDataset) -> Vec<crate::bests::SharedSelection> {
    crate::bests::build_from_dataset(
        dataset,
        &crate::bests::Options {
            scope: crate::report::Scope::AllSources,
            grad_year: Some(2027),
            limit: None,
        },
    )
}

pub(super) fn xc(
    course: &str,
    version: &str,
    unit: DistanceUnit,
    distance: u32,
) -> TestResult<EventSpecification> {
    Ok(EventSpecification {
        cross_country: Some(CrossCountryContext {
            distance: PublishedDistance::new(unit, distance)?,
            course: Some(CourseIdentity::new(course, version)?),
            measurement: CourseMeasurement::PublishedMeasured,
            conditions: None,
        }),
        ..EventSpecification::default()
    })
}
