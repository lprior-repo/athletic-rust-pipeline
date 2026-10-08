use super::*;

fn row(url: &str, name: &str) -> String {
    format!(
        r#"<li class="athlete-row data-row"><a href="{url}">{name}</a><div class="column-grad-year">2027</div></li>"#
    )
}

#[test]
fn roster_names_decode_numeric_entities_without_recursively_decoding_escaped_text() -> TestResult {
    let team = parse_team_index(TEAMS)?.teams.remove(0);
    for (encoded, expected) in [
        ("D&#x27;Arc, Jos&#233;", "José D'Arc"),
        ("A&#X1F3C3; B", "A🏃 B"),
        ("A&amp;#233; B", "A&#233; B"),
        ("A&amp;#39; B", "A&#39; B"),
        ("A&amp;quot; B", "A&quot; B"),
        ("A&#38;lt; B", "A&lt; B"),
        ("A&#xD800; B", "A&#xD800; B"),
        ("A&#99999999; B", "A&#99999999; B"),
        ("D&apos;Arc", "D'Arc"),
    ] {
        let body = row("https://wi.milesplit.com/athletes/42-example", encoded);
        let RosterVerdict::Complete { roster } = parse_roster(&body, team.clone())? else {
            return Err("a readable roster row must remain complete".into());
        };
        check!(eq; roster.athletes[0].name, expected);
        check!(eq; roster.athletes[0].athlete_id, "42");
        check!(eq; roster.athletes[0].grad_year.get(), 2027);
    }
    Ok(())
}

#[test]
fn provider_owned_athlete_hosts_preserve_profile_urls_and_roster_identity() -> TestResult {
    let team = parse_team_index(TEAMS)?.teams.remove(0);
    for host in [
        "milesplit.com",
        "www.milesplit.com",
        "wi.milesplit.com",
        "ny2.milesplit.com",
        "results.ny.milesplit.com",
    ] {
        let url = format!("https://{host}/athletes/42-example");
        let body = row(&url, "Athlete Example");
        let RosterVerdict::Complete { roster } = parse_roster(&body, team.clone())? else {
            return Err(format!(
                "a provider-owned athlete URL must retain the readable row: {url}"
            )
            .into());
        };
        check!(eq; roster.athletes[0].athlete_id, "42");
        check!(eq; roster.athletes[0].profile_url, url);
        check!(eq; roster.team, team);
    }
    Ok(())
}

#[test]
fn lookalike_hosts_cannot_supply_roster_identity() -> TestResult {
    let team = parse_team_index(TEAMS)?.teams.remove(0);
    for url in [
        "https://milesplit.com.evil.test/athletes/42-example",
        "https://notmilesplit.com/athletes/42-example",
        "https://milesplit.com@evil.test/athletes/42-example",
    ] {
        let body = row(url, "Athlete Example");
        let RosterVerdict::Quarantined {
            reason, rejected, ..
        } = parse_roster(&body, team.clone())?
        else {
            return Err("a lookalike host must not mint provider identity".into());
        };
        check!(eq; reason, RosterQuarantine::NoReadableRows);
        check!(eq; rejected[0].kind, RosterRejectionKind::MissingIdentity);
        check!(eq; rejected[0].athlete_id, None);
        let span = &body[rejected[0].row.byte_offset
            ..rejected[0].row.byte_offset + rejected[0].row.byte_length];
        check!(span.contains(url));
    }
    Ok(())
}

#[test]
fn published_roster_year_is_high_without_inventing_grade_from_the_supplied_season() -> TestResult {
    let team = parse_team_index(TEAMS)?.teams.remove(0);
    let body = row(
        "https://wi.milesplit.com/athletes/42-example",
        "Athlete Example",
    );
    let RosterVerdict::Complete { roster } = parse_roster(&body, team)? else {
        return Err("readable direct graduation year".into());
    };
    let site = Site::for_jurisdiction(UsJurisdiction::Wisconsin);
    for year in [2025, 2026, 2038] {
        let (_, athletes, _) = roster_entities(
            &roster.team,
            &roster.athletes,
            SchoolYear::new(year).ok_or("supplied season")?,
            "2026-09-20",
            &site,
        )?;
        let athlete = &athletes[0];
        check!(eq; athlete.grad_year, GradYear::CO2027);
        check!(eq; athlete.observed_grades, Vec::new());
        check!(eq;
            athlete.published_graduations,
            vec![PublishedGraduation {
                grad_year: GradYear::CO2027,
                source: SourceRef::new(
                    site.source_id(),
                    Some(format!("{}/roster", roster.team.url)),
                ),
            }]
        );
        check!(eq; athlete.derived_cohort_confidence(), Some(Confidence::HIGH));
        check!(eq; athlete.source.as_ref().ok_or("provider owner")?.id, "42");
        check!(eq;
            athlete.public_profile_urls,
            vec!["https://wi.milesplit.com/athletes/42-example"]
        );
    }
    Ok(())
}
