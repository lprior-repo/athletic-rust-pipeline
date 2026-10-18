use super::{analyse, Rules, Signals};
use census_domain::model::{CoachRole, Gender, Sport};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn parse(html: &str) -> Result<Signals, crate::CrawlError> {
    let rules = Rules::new()?;
    let mut signals = Signals::default();
    analyse(
        &rules,
        "https://school.example/athletics",
        html,
        &mut signals,
    )?;
    Ok(signals)
}

#[test]
fn school_sites_coach_name_stops_before_footer() -> TestResult {
    let html = "<h1>Cross Country</h1><p>Head Coach: Becky Remiger</p><a>T</a><footer><h2>Logo Image</h2><h2>Logo Title</h2></footer>";
    for source in [html.to_owned(), html.replace("><", ">\n<")] {
        let signals = parse(&source)?;
        check!(eq; signals.coach_hits.len(), 1);
        let hit = &signals.coach_hits[0];
        check!(eq; hit.name.as_str(), "Becky Remiger");
        check!(eq; hit.sport, Sport::CrossCountry);
        check!(eq; hit.role, CoachRole::HeadCoach);
    }
    Ok(())
}

#[test]
fn school_sites_heading_assigns_each_track_coach_own_role() -> TestResult {
    let html = "<h1>Track</h1><p>Head Coach: Matt Konrad (Sprints/Hurdles)</p><p>Assistant Coach: John Sterner (Throws)</p><p>Assistant Coach: Zach Feiber (Distance)</p><p>Assistant Coach: Monique Konrad (Jumps)</p><p>Assistant Coach: Becky Remiger (Pole Vault)</p><p>Assistant Coach: Nathan Lear (Sprints/Jumps)</p>";
    for source in [html.to_owned(), html.replace("><", ">\n<")] {
        let signals = parse(&source)?;
        check!(eq; signals.coach_hits.len(), 6);
        for (index, name) in [
            "Matt Konrad",
            "John Sterner",
            "Zach Feiber",
            "Monique Konrad",
            "Becky Remiger",
            "Nathan Lear",
        ]
        .iter()
        .enumerate()
        {
            let hit = &signals.coach_hits[index];
            check!(eq; hit.name.as_str(), *name);
            check!(eq; hit.sport, Sport::OutdoorTrack);
            let expected = if index == 0 {
                CoachRole::HeadCoach
            } else {
                CoachRole::AssistantCoach
            };
            check!(eq; hit.role, expected);
        }
    }
    Ok(())
}

#[test]
fn school_sites_adjacent_sports_keep_names_roles_and_gender_local() -> TestResult {
    let signals =
        parse("<p>Head Track Coach John Q. Public</p><p>Girls Cross Country Coach: Jane Doe</p>")?;
    check!(eq; signals.coach_hits.len(), 2);
    let first = &signals.coach_hits[0];
    check!(eq; first.name.as_str(), "John Q. Public");
    check!(eq; first.sport, Sport::OutdoorTrack);
    check!(eq; first.gender, Gender::Unknown);
    let second = &signals.coach_hits[1];
    check!(eq; second.name.as_str(), "Jane Doe");
    check!(eq; second.sport, Sport::CrossCountry);
    check!(eq; second.gender, Gender::Girls);
    Ok(())
}

#[test]
fn school_sites_inline_names_and_br_coach_rows_stay_separate() -> TestResult {
    let signals = parse("<h1>Track</h1><p><strong>Head Coach:</strong> <span>Mary Ann</span> <span>Van Buren</span><br>Assistant Coach: John Q. Public</p>")?;
    check!(eq; signals.coach_hits.len(), 2);
    check!(eq; signals.coach_hits[0].name.as_str(), "Mary Ann Van Buren");
    check!(eq; signals.coach_hits[0].role, CoachRole::HeadCoach);
    check!(eq; signals.coach_hits[1].name.as_str(), "John Q. Public");
    check!(eq; signals.coach_hits[1].role, CoachRole::AssistantCoach);
    Ok(())
}

#[test]
fn school_sites_heading_does_not_leak_into_other_sections_or_chrome() -> TestResult {
    let signals = parse("<nav><p>Track Coach Navigation Person</p></nav><section><h1>Track</h1><p>Head Coach: Jane Doe</p></section><p>Head Coach: Other Person</p><h2>Football</h2><p>Head Coach: Football Person</p><footer><p>Track Coach Footer Person</p></footer>")?;
    check!(eq; signals.coach_hits.len(), 1);
    check!(eq; signals.coach_hits[0].name.as_str(), "Jane Doe");
    Ok(())
}

#[test]
fn school_sites_cms_wrappers_preserve_ancestor_heading_for_sibling_cards() -> TestResult {
    let html = "<main><h1>Track</h1><section><article><p>Head Coach: Matt Konrad</p></article><article><p>Assistant Coach: John Sterner</p></article></section></main><p>Head Coach: Outside Person</p>";
    for source in [html.to_owned(), html.replace("><", ">\n<")] {
        let signals = parse(&source)?;
        check!(eq; signals.coach_hits.len(), 2);
        check!(eq; signals.coach_hits[0].name.as_str(), "Matt Konrad");
        check!(eq; signals.coach_hits[0].sport, Sport::OutdoorTrack);
        check!(eq; signals.coach_hits[1].name.as_str(), "John Sterner");
        check!(eq; signals.coach_hits[1].role, CoachRole::AssistantCoach);
    }
    Ok(())
}

#[test]
fn school_sites_sport_heading_requires_complete_words() -> TestResult {
    for heading in ["Package Tracking", "Cross Countrywide Programs", "XCetera"] {
        let signals = parse(&format!("<h1>{heading}</h1><p>Head Coach: Jane Doe</p>"))?;
        check!(signals.coach_hits.is_empty());
    }
    for heading in ["Track &amp; Field", "Boys Cross-Country", "Girls XC"] {
        let signals = parse(&format!("<h1>{heading}</h1><p>Head Coach: Jane Doe</p>"))?;
        check!(eq; signals.coach_hits.len(), 1);
    }
    Ok(())
}

#[test]
fn school_sites_ambiguous_multiappointment_block_is_not_admitted() -> TestResult {
    let signals =
        parse("<p>Head Track Coach Jane Doe; Assistant Girls Cross Country Coach John Smith</p>")?;
    check!(signals.coach_hits.is_empty());
    Ok(())
}

#[test]
fn school_sites_pipe_rows_do_not_mix_two_people_in_one_block() -> TestResult {
    let signals = parse("<p>Track | Boys | John Smith; Cross Country | Girls | Jane Doe</p>")?;
    check!(signals.coach_hits.is_empty());
    Ok(())
}

#[test]
fn school_sites_overlapping_patterns_for_one_person_remain_admitted() -> TestResult {
    let signals = parse("<p>John Smith | Track Coach</p>")?;
    check!(eq; signals.coach_hits.len(), 1);
    check!(eq; signals.coach_hits[0].name.as_str(), "John Smith");
    check!(eq; signals.coach_hits[0].sport, Sport::OutdoorTrack);
    Ok(())
}

#[test]
fn school_sites_one_person_pipe_appointments_keep_each_sport_and_gender() -> TestResult {
    let signals = parse("<p>Track | Boys | John Smith; Cross Country | Girls | John Smith</p>")?;
    check!(eq; signals.coach_hits.len(), 2);
    check!(signals
        .coach_hits
        .iter()
        .any(|hit| hit.sport == Sport::OutdoorTrack && hit.gender == Gender::Boys));
    check!(signals
        .coach_hits
        .iter()
        .any(|hit| hit.sport == Sport::CrossCountry && hit.gender == Gender::Girls));
    Ok(())
}

#[test]
fn school_sites_pipe_person_does_not_inherit_another_person_assistant_role() -> TestResult {
    let signals =
        parse("<p>Track | Boys | John Smith; Assistant Cross Country Coach Jane Doe</p>")?;
    check!(signals.coach_hits.is_empty());
    Ok(())
}

#[test]
fn school_sites_same_person_pipe_role_is_local_to_appointment() -> TestResult {
    let signals =
        parse("<p>Track | Boys | John Smith; Assistant Girls Cross Country Coach John Smith</p>")?;
    check!(eq; signals.coach_hits.len(), 2);
    check!(signals
        .coach_hits
        .iter()
        .any(|hit| hit.sport == Sport::OutdoorTrack && hit.role == CoachRole::HeadCoach));
    check!(signals
        .coach_hits
        .iter()
        .any(|hit| hit.sport == Sport::CrossCountry && hit.role == CoachRole::AssistantCoach));
    Ok(())
}
