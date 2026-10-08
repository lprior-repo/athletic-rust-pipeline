use census_crawl::milesplit::{
    RosterQuarantine, RosterRejection, RosterRejectionKind, SourceRowLocator, TeamRef,
};
use census_store::Store;

use super::journal::Journal;
use super::roster_is_complete;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const PHASE: &str = "milesplit_rosters_wi_2027_1";
const NEXT_PHASE: &str = "milesplit_rosters_wi_2027_2";

fn team(id: &str) -> TeamRef {
    TeamRef {
        id: id.to_string(),
        slug: format!("team-{id}"),
        url: format!("https://www.milesplit.com/teams/{id}"),
        name: format!("Team {id}"),
        city_state: "Madison, WI".to_string(),
    }
}

fn clean_row(team_id: &str) -> Journal {
    Journal {
        team_id: team_id.to_string(),
        school: None,
        year: 2027,
        observed_on: "2026-10-08".to_string(),
        capture: None,
        refusal: None,
        quarantine: None,
        rejected: Vec::new(),
        athletes: 0,
        teams: 0,
        co2027: 0,
        co2027_boys: 0,
        co2027_girls: 0,
    }
}

fn refused_row(team_id: &str) -> Journal {
    Journal {
        refusal: Some("milesplit refused the roster schema".to_string()),
        ..clean_row(team_id)
    }
}

fn quarantined_row(team_id: &str) -> Journal {
    Journal {
        quarantine: Some(RosterQuarantine::NoReadableRows),
        ..clean_row(team_id)
    }
}

fn rejected_row(team_id: &str) -> Journal {
    Journal {
        rejected: vec![RosterRejection {
            row: SourceRowLocator {
                ordinal: 4,
                byte_offset: 128,
                byte_length: 32,
            },
            athlete_id: None,
            kind: RosterRejectionKind::MissingName,
        }],
        ..clean_row(team_id)
    }
}

fn check(store: &Store, phase: &str, team: &TeamRef, complete: bool) -> TestResult {
    let actual = roster_is_complete(store, phase, team)?;
    if actual != complete {
        let id = team.id.as_str();
        return Err(format!("{id}: complete={actual}, want={complete}").into());
    }
    Ok(())
}

#[test]
fn completeness_needs_a_clean_row_or_an_authorized_refusal_without_parser_rejections() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let refused = team("52649");
    let quarantined = team("52650");
    let rejected = team("52651");
    let clean = team("52652");
    let unrecorded = team("52653");
    let clean_then_refused = team("52654");

    store.journal_done(PHASE, "wi:52649", &refused_row(&refused.id))?;
    store.journal_done(PHASE, "wi:52650", &quarantined_row(&quarantined.id))?;
    store.journal_done(PHASE, "wi:52651", &rejected_row(&rejected.id))?;
    store.journal_done(PHASE, "wi:52652", &clean_row(&clean.id))?;
    store.journal_done(PHASE, "wi:52654", &clean_row(&clean_then_refused.id))?;
    store.journal_done(
        PHASE,
        "wi:52654:refused",
        &refused_row(&clean_then_refused.id),
    )?;

    check(&store, PHASE, &refused, true)?;
    check(&store, PHASE, &quarantined, false)?;
    check(&store, PHASE, &rejected, false)?;
    check(&store, PHASE, &clean, true)?;
    check(&store, PHASE, &unrecorded, false)?;
    check(&store, PHASE, &clean_then_refused, true)?;

    check(&store, NEXT_PHASE, &clean, false)?;
    Ok(())
}
