use super::*;

const FIXTURE: &[u8] =
    include_bytes!("../../../../tests/fixtures/milesplit/profile_14222592_raw.html");

fn fixture() -> String {
    String::from_utf8(FIXTURE.to_vec()).expect("profile fixture is utf-8")
}

fn header_only(template: &str) -> String {
    format!(
        r#"<html><head><link rel="canonical" href="https://al.milesplit.com/athletes/1-a" /></head><body><h1 id="athleteName" class="athlete-name">A U Thor</h1><p class="athlete-info"><span class="current-school"><a href="https://al.milesplit.com/teams/9-some-high-school">Some High School</a></span> <span class="grad-year">Class of 2027</span></p>{template}</body></html>"#
    )
}

#[test]
fn parses_header_and_personal_records() {
    let verdict = parse_profile(&fixture()).expect("fixture parses");
    let ProfileVerdict::Complete { profile } = verdict else {
        panic!("expected complete profile, got {verdict:?}");
    };
    assert_eq!(profile.header.athlete_id, "14222592");
    assert_eq!(
        profile.header.profile_url,
        "https://al.milesplit.com/athletes/14222592-adelyn-spann"
    );
    assert_eq!(profile.header.name, "Adelyn Spann");
    assert_eq!(profile.header.school_name, "Abbeville High School");
    assert_eq!(profile.header.team_id, "38332");
    assert_eq!(profile.header.grad_year, GradYear::new(2027).unwrap());
    assert_eq!(profile.header.city_state.as_deref(), Some("Abbeville, AL"));
    assert_eq!(profile.header.gender, Gender::Girls);
    assert_eq!(profile.records.len(), 8);
    let overall: Vec<_> = profile
        .records
        .iter()
        .filter(|record| record.mode == "all")
        .collect();
    assert_eq!(overall.len(), 4);
    assert_eq!(overall[0].event_label, "100m");
    assert_eq!(overall[0].mark_label, "13.94");
    assert_eq!(overall[0].date_label.as_deref(), Some("Mar 14, 2025"));
    assert!(overall[0].state_rank.is_none());
    assert!(overall[0].national_rank.is_none());
    assert_eq!(overall[2].event_label, "Long Jump");
    assert_eq!(overall[2].mark_label, "15-0.5");
    let seasonal: Vec<_> = profile
        .records
        .iter()
        .filter(|record| record.mode == "outdoor")
        .collect();
    assert_eq!(seasonal.len(), 4);
    assert_eq!(seasonal[0].event_label, "100m");
}

#[test]
fn quarantines_pages_without_identity() {
    let verdict = parse_profile("<html><body>nothing here</body></html>")
        .expect("empty page parses to quarantine");
    assert!(matches!(
        verdict,
        ProfileVerdict::Quarantined {
            reason: ProfileQuarantine::MissingIdentity,
            ..
        }
    ));
}

#[test]
fn quarantines_header_only_pages_as_unknown_template() {
    let html = r#"<html><head><link rel="canonical" href="https://al.milesplit.com/athletes/1-a" /></head><body><h1 id="athleteName" class="athlete-name">A U Thor</h1><p class="athlete-info"><span class="current-school"><a href="https://al.milesplit.com/teams/9-some-high-school">Some High School</a></span> <span class="grad-year">Class of 2027</span></p></body></html>"#;
    let verdict = parse_profile(html).expect("header-only page parses to quarantine");
    assert!(matches!(
        verdict,
        ProfileVerdict::Quarantined {
            reason: ProfileQuarantine::UnknownTemplate,
            ..
        }
    ));
}

#[test]
fn rejects_rows_without_marks_but_keeps_the_rest() {
    let html = r#"<html><head><link rel="canonical" href="https://al.milesplit.com/athletes/1-a" /></head><body><h1 id="athleteName" class="athlete-name">A U Thor</h1><p class="athlete-info"><span class="current-school"><a href="https://al.milesplit.com/teams/9-some-high-school">Some High School</a></span> <span class="grad-year">Class of 2027</span></p><table id="prTable"><tbody><tr data-pr-mode="all" data-pr-level="all"><td class="pr-td-event">1600m</td><td class="pr-td-mark"></td><td class="pr-td-rank"></td><td class="pr-td-rank"></td><td class="pr-td-date">Apr 1, 2025</td></tr><tr data-pr-mode="all" data-pr-level="all"><td class="pr-td-event">800m</td><td class="pr-td-mark">2:25.90</td><td class="pr-td-rank"></td><td class="pr-td-rank"></td><td class="pr-td-date">May 2, 2025</td></tr></tbody></table></body></html>"#;
    let verdict = parse_profile(html).expect("mixed rows parse");
    let ProfileVerdict::Partial { profile, rejected } = verdict else {
        panic!("expected partial profile, got {verdict:?}");
    };
    assert_eq!(profile.records.len(), 1);
    assert_eq!(profile.records[0].event_label, "800m");
    assert_eq!(rejected.len(), 1);
    assert_eq!(rejected[0].ordinal, 0);
    assert_eq!(
        rejected[0].kind,
        ProfileRowRejectionKind::MissingMark
    );
}

#[test]
fn every_table_row_reaches_the_verdict() {
    let html = fixture();
    let expected = html.matches("<tr data-pr-mode=\"").count();
    let verdict = parse_profile(&html).expect("fixture parses");
    let (records, rejected) = match verdict {
        ProfileVerdict::Complete { profile } => (profile.records, vec![]),
        ProfileVerdict::Partial { profile, rejected } => (profile.records, rejected),
        ProfileVerdict::Quarantined { reason, .. } => {
            panic!("a populated table is never quarantined, got {reason:?}")
        }
    };
    assert!(expected > 0, "the fixture must contain scoped rows");
    assert!(
        rejected.is_empty(),
        "no row is dropped silently: {rejected:?}"
    );
    assert_eq!(
        records.len(),
        expected,
        "every scoped row is captured, none invented"
    );
    for mode in ["all", "outdoor"] {
        for event in ["100m", "100mH", "Long Jump", "Triple Jump"] {
            assert!(
                records
                    .iter()
                    .any(|record| record.mode == mode && record.event_label == event),
                "missing {mode}/{event}"
            );
        }
    }
}

#[test]
fn parsing_twice_gives_the_same_profile() {
    let html = fixture();
    let first = parse_profile(&html).expect("first parse");
    let second = parse_profile(&html).expect("second parse");
    assert_eq!(format!("{first:?}"), format!("{second:?}"));
}

#[test]
fn quarantines_pages_without_a_school_or_grad_year() {
    let no_school = r#"<html><head><link rel="canonical" href="https://al.milesplit.com/athletes/1-a" /></head><body><h1 id="athleteName" class="athlete-name">A U Thor</h1></body></html>"#;
    assert!(matches!(
        parse_profile(no_school).expect("school-less page parses"),
        ProfileVerdict::Quarantined {
            reason: ProfileQuarantine::MissingSchool,
            ..
        }
    ));
    let no_grad = header_only("<table id=\"prTable\"><tbody></tbody></table>")
        .replace(" <span class=\"grad-year\">Class of 2027</span>", "");
    assert!(matches!(
        parse_profile(&no_grad).expect("grad-less page parses"),
        ProfileVerdict::Quarantined {
            reason: ProfileQuarantine::MissingGraduationYear,
            ..
        }
    ));
    let bad_grad = header_only("<table id=\"prTable\"><tbody></tbody></table>")
        .replace("Class of 2027", "Class of 1999");
    assert!(matches!(
        parse_profile(&bad_grad).expect("bad-grad page parses"),
        ProfileVerdict::Quarantined {
            reason: ProfileQuarantine::InvalidGraduationYear,
            ..
        }
    ));
}

#[test]
fn an_empty_table_is_a_complete_profile_with_no_records() {
    let html = header_only("<table id=\"prTable\"><tbody></tbody></table>");
    let verdict = parse_profile(&html).expect("empty table parses");
    let ProfileVerdict::Complete { profile } = verdict else {
        panic!("an athlete with no PRs yet is known-empty, got {verdict:?}");
    };
    assert!(profile.records.is_empty());
    assert_eq!(profile.header.name, "A U Thor");
}
