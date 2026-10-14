use super::*;
use census_crawl::milesplit::parse_roster;
use census_domain::UsJurisdiction;
use std::num::NonZeroU32;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct LegacyBytes {
    canonical_bytes: String,
    journal_bytes: String,
    facts_digest: String,
    effect_digest: String,
}

fn input() -> CrawlResult<(TeamRef, RosterOutcome)> {
    let team = TeamRef {
        id: "52649".to_string(),
        slug: "example".to_string(),
        url: "https://wi.milesplit.com/teams/52649-example".to_string(),
        name: "Example High School".to_string(),
        city_state: "Madison, WI".to_string(),
    };
    let body = concat!(
        "é🏃<link rel=\"canonical\" href=\"https://wi.milesplit.com/teams/52649-example/roster\">",
        "<ul id=\"rosterDataset\">",
        "<li class=\"athlete-row data-row\"><a href=\"https://wi.milesplit.com/athletes/1-runner\">Runner, Alice</a><div class=\"column-gender\">f</div><div class=\"column-grad-year\">2027</div><div class=\"data-point\" data-season-id=\"1\"><svg class=\"icon icon-yes\"></svg></div></li>",
        "<li class=\"athlete-row data-row\"><a href=\"https://wi.milesplit.com/athletes/2-runner\">Runner, Missing</a><div class=\"column-grad-year\"></div></li>",
        "<li class=\"athlete-row data-row\"><a href=\"https://wi.milesplit.com/athletes/3-runner\">Runner, Bob</a><div class=\"column-gender\">m</div><div class=\"column-grad-year\">2028</div><div class=\"data-point\" data-season-id=\"1\"><svg class=\"icon icon-yes\"></svg></div></li>",
        "</ul>"
    );
    let verdict = parse_roster(body, team.clone())?;
    Ok((
        team.clone(),
        RosterOutcome {
            verdict,
            capture: FetchOutcome {
                url: format!("{}/roster", team.url),
                response_url: None,
                method: "GET".to_string(),
                status: 200,
                content_digest: format!("{:x}", Sha256::digest(body.as_bytes())),
                bytes: body.len(),
                fetched_at: "2026-10-07T12:00:00Z".to_string(),
                from_cache: false,
                content_type: Some("text/html".to_string()),
                body: body.as_bytes().to_vec(),
            },
        },
    ))
}

fn legacy_bytes() -> Result<LegacyBytes, Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let (team, read) = input()?;
    let run = RosterRun {
        site: Site::for_jurisdiction(UsJurisdiction::Wisconsin),
        observed_on: "2026-10-07",
        school_year: SchoolYear::DEFAULT,
        refresh: true,
        revision: NonZeroU32::new(1).ok_or("invalid fixture revision")?,
    };
    let roster = read
        .verdict
        .roster()
        .ok_or("missing legacy fixture roster")?;
    let window = Window::next(roster, 0, run.observed_on)?.ok_or("missing fixture window")?;
    let records = Records::of(&window, &run.site, run.school_year)?;
    let mut counts = Counts::new()?;
    counts.include(records.members(), records.teams())?;
    check!(matches!(&read.verdict, RosterVerdict::Partial { .. }));
    check!(eq; read.verdict.rejections().len(), 1);
    let rejected = read
        .verdict
        .rejections()
        .first()
        .ok_or("missing fixture rejection")?;
    check!(eq; rejected.row.ordinal, 1);
    check!(eq; rejected.athlete_id.as_deref(), Some("2"));
    check!(eq; rejected.kind, milesplit::RosterRejectionKind::MissingGraduationYear);
    check!(eq; read.verdict.unfinished(), None);
    check!(eq; (records.athletes.len(), records.teams.len(), records.athlete_observations.len()), (2, 2, 2));
    check!(eq; records.athletes.iter().map(|row| row.gender).collect::<Vec<_>>(), vec![census_domain::model::Gender::Girls, census_domain::model::Gender::Boys]);
    check!(eq; records.athletes.iter().map(|row| row.grad_year.get()).collect::<Vec<_>>(), vec![2027, 2028]);
    check!(eq; (counts.athletes, counts.co2027, counts.boys, counts.girls, counts.teams()), (2, 1, 0, 1, 2));
    let canonical_bytes = serde_json::to_string(&(
        &records.school,
        &records.teams,
        &records.athletes,
        &records.school_observation,
        &records.athlete_observations,
    ))?;
    let journal_bytes =
        serde_json::to_string(&roster_journal(&team, &read, Some(&records), &run, &counts))?;
    let facts_digest = records.stage(&store, &mut store.write_batch())?;
    let effect_digest = roster_digest(&facts_digest, &team, &run, &read, 0)?;
    Ok(LegacyBytes {
        canonical_bytes,
        journal_bytes,
        facts_digest,
        effect_digest,
    })
}

#[test]
fn positive_roster_preserves_prechange_canonical_journal_and_effect_bytes() -> TestResult {
    let expected: LegacyBytes = serde_json::from_str(include_str!("legacy_roster_bytes.json"))?;
    let actual = legacy_bytes()?;
    check!(eq; actual.canonical_bytes, expected.canonical_bytes);
    check!(eq; actual.journal_bytes, expected.journal_bytes);
    check!(eq; actual.facts_digest, expected.facts_digest);
    check!(eq; actual.effect_digest, expected.effect_digest);
    Ok(())
}
