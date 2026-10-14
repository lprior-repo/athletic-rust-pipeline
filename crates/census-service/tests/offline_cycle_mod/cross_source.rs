use super::readback::{assert_expected_bests, ExpectedBest};
use super::*;

const SECOND_URL: &str = "https://example.test/second-provider/outdoor-results";
const INDOOR_URL: &str = "https://example.test/second-provider/indoor-results";
const CAPTURE_DATE: &str = "2026-06-01T12:00:00Z";
const OUTDOOR_DATE: &str = "2026-05-03";
const INDOOR_DATE: &str = "2026-02-07";

struct Observation {
    kind: EventKind,
    label: &'static str,
    sport: Sport,
    date: &'static str,
    url: &'static str,
    key: &'static str,
    seconds: &'static str,
}

fn add_observations(corpus: &mut Corpus, observation: Observation) -> TestResult {
    let source = SourceRef::new("second_provider", Some(observation.url.to_string()));
    let evidence = Evidence::parsed(source.clone(), CAPTURE_DATE);
    let mut meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        format!("Second Provider {}", observation.key),
        observation.date,
        CompetitionLevel::Invitational,
    );
    meet.sports.push(observation.sport);
    meet.source_urls.push(observation.url.to_string());
    meet.evidence.push(evidence.clone());
    let mut event = CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: observation.kind,
            gender: Gender::Boys,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?;
    event.source_labels.push(SourceEventLabel {
        source,
        label: observation.label.to_string(),
    });
    event.evidence.push(evidence.clone());
    for athlete in &mut corpus.athletes {
        let original = corpus
            .performances
            .iter()
            .find(|performance| performance.athlete == athlete.id && performance.date == MEET_DATE)
            .ok_or("baseline athlete result is missing")?;
        let mut performance = original.clone();
        let owner = SourceIdentity::new(
            SourceNamespace::Other("second_provider".to_string()),
            format!("second-{}", athlete.id),
        );
        athlete.add_identity(owner.clone());
        performance.source_key = format!("{}-{}", observation.key, athlete.id);
        performance.id = CanonicalPerformance::mint(
            &athlete.id,
            &meet.id,
            &event.id,
            observation.date,
            &performance.source_key,
        );
        performance.team = corpus
            .teams
            .iter()
            .find(|team| team.school == athlete.school && team.sport == observation.sport)
            .ok_or("matching fixture team is missing")?
            .id
            .clone();
        performance.meet = meet.id.clone();
        performance.event = event.id.clone();
        performance.date = observation.date.to_string();
        performance.mark = Mark::TimeSeconds(ExactSeconds::parse(observation.seconds)?);
        performance.source_athlete = Some(owner);
        performance.evidence = vec![evidence.clone()];
        corpus.performances.push(performance);
    }
    corpus.meets.push(meet);
    corpus.events.push(event);
    Ok(())
}

fn cross_source_corpus() -> TestResult<Corpus> {
    let mut corpus = build_corpus(2)?;
    let indoor_teams: Vec<_> = corpus
        .teams
        .iter()
        .map(|team| {
            let mut indoor = team.clone();
            indoor.id = Id::mint("team", &[team.school.as_str(), "indoor", "m", "2025"]);
            indoor.sport = Sport::IndoorTrack;
            indoor.evidence = vec![Evidence::parsed(
                SourceRef::new("second_provider", Some(INDOOR_URL.to_string())),
                CAPTURE_DATE,
            )];
            indoor
        })
        .collect();
    corpus.teams.extend(indoor_teams);
    for observation in [
        Observation {
            kind: EventKind::Track100m,
            label: "100 Meter Dash",
            sport: Sport::OutdoorTrack,
            date: OUTDOOR_DATE,
            url: SECOND_URL,
            key: "second-100",
            seconds: "11.75",
        },
        Observation {
            kind: EventKind::Track400m,
            label: "400 Meter Dash",
            sport: Sport::OutdoorTrack,
            date: OUTDOOR_DATE,
            url: SECOND_URL,
            key: "outdoor-400",
            seconds: "51.50",
        },
        Observation {
            kind: EventKind::Track400m,
            label: "400 Meter Dash",
            sport: Sport::IndoorTrack,
            date: INDOOR_DATE,
            url: INDOOR_URL,
            key: "indoor-400",
            seconds: "52.25",
        },
    ] {
        add_observations(&mut corpus, observation)?;
    }
    Ok(corpus)
}

fn expected_bests(corpus: &Corpus) -> TestResult<Vec<ExpectedBest>> {
    let mut expected = Vec::new();
    for athlete in &corpus.athletes {
        for (key, event, mark, seconds, nanoseconds, date, url, surface, sources, marks) in [
            (
                "second-100",
                "Track100m",
                "11.75",
                11.75,
                "11750000000",
                "2026-05-03",
                SECOND_URL,
                "outdoor",
                2.0,
                "2",
            ),
            (
                "outdoor-400",
                "Track400m",
                "51.50",
                51.5,
                "51500000000",
                "2026-05-03",
                SECOND_URL,
                "outdoor",
                1.0,
                "1",
            ),
            (
                "indoor-400",
                "Track400m",
                "52.25",
                52.25,
                "52250000000",
                "2026-02-07",
                INDOOR_URL,
                "indoor",
                1.0,
                "1",
            ),
        ] {
            let source_key = format!("{key}-{}", athlete.id);
            let performance = corpus
                .performances
                .iter()
                .find(|performance| {
                    performance.athlete == athlete.id && performance.source_key == source_key
                })
                .ok_or("explicit expected winner is missing from fixture")?;
            expected.push(ExpectedBest {
                performance: performance.clone(),
                event,
                mark,
                seconds,
                nanoseconds,
                date,
                url,
                surface,
                sources,
                marks,
            });
        }
    }
    Ok(expected)
}

#[test]
fn the_offline_cycle_retains_each_event_and_surface_pr_with_the_cross_source_winner() -> TestResult
{
    let tmpdir = TempDir::new()?;
    let data_dir = tmpdir.path().join("store");
    std::fs::create_dir_all(&data_dir)?;
    let corpus = cross_source_corpus()?;
    check!(eq; corpus.athletes.len(), 10, "same ten canonical fixture athletes");
    check!(eq; corpus.performances.len(), 40, "four observations per athlete");
    let expected = expected_bests(&corpus)?;
    check!(eq; expected.len(), 30, "three eligible comparison classes per athlete");
    {
        let store = Store::open(&data_dir)?;
        corpus.append(&store)?;
    }
    let output_path = tmpdir.path().join("publication");
    let output = run_cycle(&data_dir, &output_path)?;
    check!(
        output.status.success(),
        "offline run must exit 0, got {:?}",
        output.status
    );
    let published = census_report::workbook::publication::current_workbook(&output_path)?;
    check!(eq; xlsx_rows(&published, "Athletes")?, 11, "one row per canonical athlete");
    let expected_ids: HashSet<_> = corpus
        .athletes
        .iter()
        .map(|athlete| athlete.id.to_string())
        .collect();
    check!(eq; unique_athlete_ids(&published)?, expected_ids, "no athlete is lost or duplicated");
    assert_performance_inventory(&published, &corpus)?;
    assert_expected_bests(&published, &expected)?;
    let status = verify(&data_dir, &published)?;
    check!(
        status.success(),
        "complete publication verification must exit 0, got {status:?}"
    );
    Ok(())
}

fn assert_performance_inventory(path: &Path, corpus: &Corpus) -> TestResult {
    let mut book = calamine::open_workbook_auto(path)?;
    let range = book.worksheet_range("Performances_001")?;
    let mut rows = range.rows();
    let headers = rows.next().ok_or("performances header missing")?;
    let column = headers
        .iter()
        .position(|cell| cell == "Canonical Result ID")
        .ok_or("performance identity column missing")?;
    let mut actual: Vec<String> = rows
        .map(|row| {
            row.get(column)
                .map(ToString::to_string)
                .ok_or("performance identity missing")
        })
        .collect::<Result<_, _>>()?;
    let mut expected: Vec<String> = corpus
        .performances
        .iter()
        .map(|performance| performance.id.to_string())
        .collect();
    actual.sort();
    expected.sort();
    check!(eq; actual, expected, "every source observation is retained exactly once");
    Ok(())
}
