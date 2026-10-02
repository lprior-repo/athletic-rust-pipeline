use super::*;

fn row(url: &str, name: &str) -> String {
    format!(
        r#"<li class="athlete-row data-row"><a href="{url}">{name}</a><div class="column-grad-year">2027</div></li>"#
    )
}

#[test]
fn roster_names_decode_numeric_entities_without_recursively_decoding_escaped_text() {
    let team = parse_team_index(TEAMS).unwrap().remove(0);
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
        let RosterVerdict::Complete { roster } = parse_roster(&body, team.clone()).unwrap() else {
            panic!("a readable roster row must remain complete");
        };
        assert_eq!(roster.athletes[0].name, expected);
        assert_eq!(roster.athletes[0].athlete_id, "42");
        assert_eq!(roster.athletes[0].grad_year.get(), 2027);
    }
}

#[test]
fn provider_owned_athlete_hosts_preserve_profile_urls_and_roster_identity() {
    let team = parse_team_index(TEAMS).unwrap().remove(0);
    for host in [
        "milesplit.com",
        "www.milesplit.com",
        "wi.milesplit.com",
        "ny2.milesplit.com",
        "results.ny.milesplit.com",
    ] {
        let url = format!("https://{host}/athletes/42-example");
        let body = row(&url, "Athlete Example");
        let RosterVerdict::Complete { roster } = parse_roster(&body, team.clone()).unwrap() else {
            panic!("a provider-owned athlete URL must retain the readable row: {url}");
        };
        assert_eq!(roster.athletes[0].athlete_id, "42");
        assert_eq!(roster.athletes[0].profile_url, url);
        assert_eq!(roster.team, team);
    }
}

#[test]
fn lookalike_hosts_cannot_supply_roster_identity() {
    let team = parse_team_index(TEAMS).unwrap().remove(0);
    for url in [
        "https://milesplit.com.evil.test/athletes/42-example",
        "https://notmilesplit.com/athletes/42-example",
        "https://milesplit.com@evil.test/athletes/42-example",
    ] {
        let body = row(url, "Athlete Example");
        let RosterVerdict::Quarantined { reason, rejected } =
            parse_roster(&body, team.clone()).unwrap()
        else {
            panic!("a lookalike host must not mint provider identity");
        };
        assert_eq!(reason, RosterQuarantine::NoReadableRows);
        assert_eq!(rejected[0].kind, RosterRejectionKind::MissingIdentity);
        assert_eq!(rejected[0].athlete_id, None);
        let span = &body[rejected[0].row.byte_offset
            ..rejected[0].row.byte_offset + rejected[0].row.byte_length];
        assert!(span.contains(url));
    }
}
