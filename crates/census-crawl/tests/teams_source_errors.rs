#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use census_crawl::{milesplit::parse_team_index, CollectionDisposition};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn dc_published_national_link_retains_school_identity() -> TestResult {
    let read = parse_team_index(include_str!("fixtures/milesplit/dc_team_www.html"))?;
    check!(eq; read.errors, 0);
    check!(eq; read.disposition, CollectionDisposition::Complete);
    let team = read.teams.first().ok_or("DC row must survive")?;
    check!(eq; team.id, "3305");
    check!(eq; team.url, "https://www.milesplit.com/teams/3305-archbishop-carroll-high-school-dc");
    check!(eq; team.city_state, "Washington, DC, USA");
    Ok(())
}

#[test]
fn la_published_national_link_retains_school_identity() -> TestResult {
    let read = parse_team_index(include_str!("fixtures/milesplit/la_team_www.html"))?;
    check!(eq; read.errors, 0);
    let team = read.teams.first().ok_or("LA row must survive")?;
    check!(eq; team.id, "29171");
    check!(eq; team.city_state, "Covington, LA");
    Ok(())
}

#[test]
fn tx_published_id_only_link_does_not_invent_a_slug() -> TestResult {
    let read = parse_team_index(include_str!("fixtures/milesplit/tx_team_id_only.html"))?;
    check!(eq; read.errors, 0);
    let team = read.teams.first().ok_or("TX row must survive")?;
    check!(eq; team.id, "73074");
    check!(eq; team.slug, "");
    check!(eq; team.url, "https://tx.milesplit.com/teams/73074");
    check!(eq; team.name, "Alicia R. Chacón International school");
    Ok(())
}

#[test]
fn ma_foreign_directory_row_is_owed_with_published_country() -> TestResult {
    let read = parse_team_index(include_str!("fixtures/milesplit/ma_team_foreign.html"))?;
    check!(read.teams.is_empty());
    check!(eq; read.errors, 1);
    check!(eq; read.disposition, CollectionDisposition::Quarantined);
    let reason = read.unfinished.first().ok_or("foreign row must be owed")?;
    check!(reason.contains("foreign-directory"), "{reason}");
    check!(reason.contains("MA, ENG"), "{reason}");
    check!(reason.contains("15373"), "{reason}");
    Ok(())
}

#[test]
fn sd_foreign_directory_rows_are_each_owed_with_published_country() -> TestResult {
    let read = parse_team_index(include_str!("fixtures/milesplit/sd_teams_foreign.html"))?;
    check!(read.teams.is_empty());
    check!(eq; read.errors, 3);
    check!(eq; read.unfinished.len(), 3);
    for (reason, id) in read.unfinished.iter().zip(["32427", "32422", "32432"]) {
        check!(reason.contains("foreign-directory"), "{reason}");
        check!(reason.contains("GRN"), "{reason}");
        check!(reason.contains(id), "{reason}");
    }
    Ok(())
}

#[test]
fn ct_recorded_directory_keeps_every_named_school_table() -> TestResult {
    let html = match std::env::var("CENSUS_CIAC_REPLAY") {
        Ok(path) => std::fs::read_to_string(path)?,
        Err(std::env::VarError::NotPresent) => {
            include_str!("fixtures/ciac/directory.html").to_string()
        }
        Err(error) => return Err(error.into()),
    };
    let published = html.matches("<table class='DirectoryStaffTable'>").count();
    check!(published > 0, "a replay must publish school tables");
    let parsed = census_crawl::ciac::parse_directory(&html);
    check!(eq; parsed.len(), published);
    let empty: Vec<_> = parsed
        .iter()
        .filter(|(_, table)| table.rows.is_empty())
        .map(|(name, _)| name.as_str())
        .collect();
    eprintln!(
        "published={published}; retained={}; no published named sport staff={empty:?}",
        parsed.len()
    );
    Ok(())
}
