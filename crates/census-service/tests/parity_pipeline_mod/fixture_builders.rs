use crate::{common, listing};

use std::collections::BTreeSet;

use anyhow::{bail, ensure, Context, Result};
use census_crawl::{athleticlive, athleticlive_athletes, milesplit};
use census_domain::model::{
    CanonicalMeet, CompetitionLevel, Evidence, SourceIdentity, SourceNamespace, SourceRef,
};

use super::constants;
use super::fixtures::Corpus;

pub fn milesplit_roster(corpus: &mut Corpus) -> Result<()> {
    let mut index_body = None;
    let mut roster: Option<(String, String)> = None;
    for path in listing::fixtures("milesplit")? {
        let name = listing::file_name(&path)?;
        let body = common::fixture("milesplit", &name)?;
        if name == "wi_teams_index.html" {
            index_body = Some(body);
        } else if let Some(team_id) = name
            .strip_prefix("wi_roster_")
            .and_then(|rest| rest.strip_suffix(".html"))
        {
            roster = Some((team_id.to_string(), body));
        } else if super::milesplit_fixtures::validate_result_fixture(&name, &body)?
            || matches!(
                name.as_str(),
                "oh_teams_index.html"
                    | "oh_results_index.html"
                    | "oh_roster_10002_mason.html"
                    | "al_teams_index.html"
            )
        {
            continue;
        } else {
            bail!("{name} is not a known milesplit fixture");
        }
    }
    let index_body = index_body.context("the milesplit corpus carries no team index")?;
    let (team_id, roster_body) = roster.context("the milesplit corpus carries no roster")?;
    let teams = milesplit::parse_team_index(&index_body)?.teams;
    let team = teams
        .iter()
        .find(|team| team.id == team_id)
        .with_context(|| format!("the team index does not list team {team_id}"))?
        .clone();
    let verdict = milesplit::parse_roster(&roster_body, team)?;
    let parsed = verdict
        .roster()
        .context("the roster fixture was quarantined")?;
    ensure!(
        !parsed.athletes.is_empty(),
        "the roster fixture parses to no athletes"
    );
    let site = milesplit::Site::for_jurisdiction(UsJurisdiction::Wisconsin);
    let (school, athletes, school_teams) = milesplit::roster_entities(
        &parsed.team,
        &parsed.athletes,
        constants::SCHOOL_YEAR,
        constants::OBSERVED_ON,
        &site,
    )?;
    corpus.schools.push(school);
    corpus.athletes.extend(athletes);
    corpus.teams.extend(school_teams);
    Ok(())
}

pub fn athleticlive_meets(corpus: &mut Corpus) -> Result<()> {
    for path in listing::fixtures("athleticlive")? {
        let name = listing::file_name(&path)?;
        ensure!(
            name == "meets-sample.csv",
            "{name} is not a known athleticlive fixture"
        );
        let rows = athleticlive::parse_meets_csv(&common::fixture("athleticlive", &name)?)?;
        ensure!(!rows.is_empty(), "{name}: no meet rows");
        let meets = athleticlive::build_meets(
            &rows,
            constants::OBSERVED_ON,
            constants::SOURCE_ATHLETICLIVE_MEETS,
        );
        ensure!(!meets.is_empty(), "{name}: no canonical meet");
        corpus.meets.extend(meets);
    }
    Ok(())
}

pub fn athleticlive_athletes(corpus: &mut Corpus) -> Result<()> {
    for path in listing::fixtures("athleticlive_athletes")? {
        let name = listing::file_name(&path)?;
        ensure!(
            name == "athlete-list-sample.json",
            "{name} is not a known athleticlive_athletes fixture"
        );
        let document: serde_json::Value =
            serde_json::from_str(&common::fixture("athleticlive_athletes", &name)?)?;
        let sources: Vec<serde_json::Value> = document["hits"]["hits"]
            .as_array()
            .context("the fixture carries no `hits.hits` array")?
            .iter()
            .map(|hit| hit["_source"].clone())
            .collect();
        let hits: Vec<athleticlive_athletes::AthleteHit> =
            serde_json::from_value(serde_json::Value::Array(sources))
                .context("the fixture's hits decode as athlete rows")?;
        ensure!(!hits.is_empty(), "{name}: no athlete rows");
        let meet_ids: BTreeSet<u64> = hits.iter().filter_map(|hit| hit.meet_id()).collect();
        ensure!(
            meet_ids.len() == 1,
            "{name}: the rows name {} meets; the capture is a single meet",
            meet_ids.len()
        );
        let meet_id = *meet_ids.iter().next().context("no AthleticLIVE meet id")?;

        let mut meet = CanonicalMeet::new(
            Some(UsJurisdiction::Kansas),
            "Abilene Invitational",
            "2025-04-25",
            CompetitionLevel::Invitational,
        );
        meet.source_identities.push(SourceIdentity::new(
            SourceNamespace::TimerMeet {
                provider: "reddirt".to_string(),
            },
            meet_id.to_string(),
        ));
        meet.evidence.push(Evidence::parsed(
            SourceRef::new(constants::SOURCE_ATHLETICLIVE_MEETS, None),
            constants::OBSERVED_ON,
        ));
        let selection = athleticlive_athletes::meet_targets(
            std::slice::from_ref(&meet),
            &[UsJurisdiction::Kansas],
        );
        let by_id: HashMap<u64, &athleticlive_athletes::MeetTarget> = selection
            .targets
            .iter()
            .map(|target| (target.athleticlive_meet_id, target))
            .collect();
        ensure!(
            by_id.contains_key(&meet_id),
            "the corpus holds no target for AthleticLIVE meet {meet_id}"
        );
        let entities = athleticlive_athletes::build_entities(
            &hits,
            &by_id,
            constants::OBSERVED_ON,
            constants::SCHOOL_YEAR,
        );
        ensure!(
            entities.rows == hits.len(),
            "the adapter examined {} of {} athlete rows",
            entities.rows,
            hits.len()
        );
        ensure!(
            !entities.athletes.is_empty(),
            "{name}: the rows mint no athlete"
        );
        ensure!(
            entities.rows_with_grade > 0,
            "{name}: no row carries a grade"
        );
        corpus.meets.push(meet);
        corpus.schools.extend(entities.schools);
        corpus.teams.extend(entities.teams);
        corpus.athletes.extend(entities.athletes);
    }
    Ok(())
}

use census_domain::UsJurisdiction;
use std::collections::HashMap;
