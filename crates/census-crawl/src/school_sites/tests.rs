use super::parse::{analyse, base_domain, clean_person, rank_link, resolve_href, Rules, Signals};
use super::{parse_queue, site_root, slug};
use census_domain::model::{CoachRole, Gender, Sport};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn page() -> String {
    let mut html = String::new();
    html.push_str(
        "<!doctype html><html><head><title>Example High School Athletics</title></head><body>",
    );
    html.push_str("<a href=\"mailto:Coach.Smith@Example.ORG?subject=hello\">Email</a>");
    html.push_str("<a href=\"/staff-directory\">Staff Directory</a>");
    html.push_str("<a href=\"https://www.facebook.com/examplehs\">Facebook</a>");
    html.push_str("<a href=\"https://bigteams.com/example\">Team page</a>");
    html.push_str("<p>Head Track Coach John Q. Public</p>");
    html.push_str("<p>Girls Cross Country Coach: Jane Doe</p>");
    html.push_str("<p>Athletic Director: Bob Jones</p>");
    html.push_str("<table><tr><th>Sport</th><th>Coach</th><th>E-mail</th></tr>");
    html.push_str("<tr><td>Cross Country - Girls</td><td>Mary Ann Lee</td><td>mlee@example.org</td></tr></table>");
    html.push_str("</body></html>");
    html
}

#[test]
fn email_sanitizer_normalizes_and_rejects() -> TestResult {
    let rules = Rules::new()?;
    let cases = [
        ("Athletics@School.ORG", Some("athletics@school.org")),
        ("x@y.org\\&quot;&gt;", Some("x@y.org")),
        ("info@school.edu.", Some("info@school.edu")),
        ("a\u{200b}b@school.org", Some("ab@school.org")),
        ("bad@localhost", None),
        ("noreply@school.zz", Some("noreply@school.zz")),
        ("a..b@school.org", None),
        ("two@at@school.org", None),
        ("nums@123.456", None),
        ("a b@school.org", None),
        ("mailto:Coach@School.org", Some("coach@school.org")),
        ("", None),
    ];
    for (raw, expected) in cases {
        check!(eq; rules.sanitize_email(raw).as_deref(), expected, "raw={raw}");
    }
    Ok(())
}

fn coach_rows(signals: &Signals) -> Vec<(&str, Sport, CoachRole, Gender, Option<&str>)> {
    signals
        .coach_hits
        .iter()
        .map(|hit| {
            (
                hit.name.as_str(),
                hit.sport,
                hit.role,
                hit.gender,
                hit.email.as_deref(),
            )
        })
        .collect()
}

#[test]
fn page_analysis_extracts_emails_hits_and_tables() -> TestResult {
    let rules = Rules::new()?;
    let mut signals = Signals::default();
    analyse(
        &rules,
        "https://example.k12.us/athletics",
        &page(),
        &mut signals,
    );
    signals.finish();
    check!(eq; signals.emails, vec![
        "coach.smith@example.org".to_string(),
        "mlee@example.org".to_string()
    ]);
    check!(eq; signals.pages, vec!["https://example.k12.us/athletics".to_string()]);
    let coaches = coach_rows(&signals);
    check!(coaches.contains(&(
        "John Q. Public Girls",
        Sport::CrossCountry,
        CoachRole::HeadCoach,
        Gender::Girls,
        None
    )));
    check!(coaches.contains(&(
        "Jane Doe",
        Sport::CrossCountry,
        CoachRole::HeadCoach,
        Gender::Girls,
        None
    )));
    check!(coaches.contains(&(
        "John Q. Public",
        Sport::CrossCountry,
        CoachRole::HeadCoach,
        Gender::Girls,
        None
    )));
    check!(coaches.contains(&(
        "Mary Ann Lee",
        Sport::CrossCountry,
        CoachRole::HeadCoach,
        Gender::Girls,
        Some("mlee@example.org")
    )));
    Ok(())
}

#[test]
fn page_analysis_extracts_director_hits() -> TestResult {
    let rules = Rules::new()?;
    let mut signals = Signals::default();
    analyse(
        &rules,
        "https://example.k12.us/athletics",
        &page(),
        &mut signals,
    );
    signals.finish();
    check!(eq; signals.ad_hits.len(), 2);
    check!(signals
        .ad_hits
        .iter()
        .all(|hit| hit.name.contains("Bob Jones")));
    Ok(())
}

#[test]
fn junk_context_is_rejected() -> TestResult {
    let rules = Rules::new()?;
    let html =
        "<p>Track Coach John Doe passed away in 2019, a former coach remembered by alumni.</p>";
    let mut signals = Signals::default();
    analyse(&rules, "https://example.org/memory", html, &mut signals);
    check!(eq; signals.coach_hits.len(), 0);
    Ok(())
}

#[test]
fn person_cleaning_trims_titles_and_initials() -> TestResult {
    let rules = Rules::new()?;
    check!(eq; clean_person(&rules, "Coach John Q. Public").as_deref(), Some("John Q. Public"));
    check!(eq; clean_person(&rules, "Head Coach Jane Doe").as_deref(), Some("Jane Doe"));
    check!(eq; clean_person(&rules, "John Doe Coach").as_deref(), Some("John Doe"));
    check!(eq; clean_person(&rules, "Athletics Director Bob Jones").as_deref(), Some("Bob Jones"));
    check!(eq; clean_person(&rules, "Athletic Director"), None);
    check!(eq; clean_person(&rules, "Jane"), None);
    Ok(())
}

#[test]
fn navigation_rules_match_the_prototype() -> TestResult {
    check!(eq; base_domain("www.example.k12.mo.us"), "k12.mo.us");
    check!(eq; base_domain("www.schools.example.org"), "example.org");
    check!(eq; base_domain("example.com"), "example.com");
    check!(eq; rank_link("https://x/coaches", "Coaching Staff"), 0);
    check!(eq; rank_link("https://x/athletics", "Athletics"), 1);
    check!(eq; rank_link("https://x/staff", "Staff Directory"), 2);
    check!(eq; rank_link("https://x/about", "About"), 3);
    check!(eq; resolve_href("https://x.org/home", "/coaches"), "https://x.org/home/coaches");
    check!(eq; resolve_href("https://x.org", "https://y.org/z"), "https://y.org/z");
    check!(eq; site_root("https://x.org/pages/home"), "https://x.org");
    check!(eq; slug("Rock Bridge High School"), "rock-bridge-high-school");
    check!(eq; slug("Bishop DuBourg (St. Louis)"), "bishop-dubourg-st-louis");
    Ok(())
}

#[test]
fn queue_parsing_keeps_valid_rows_and_counts_rejections() -> TestResult {
    let text = "{\"state\":\"MO\",\"name\":\"Rock Bridge High School\",\"website\":\"https://rbhs.example.org\"}\n\
                not json\n\
                {\"state\":\"MO\",\"name\":\"Second\",\"website\":\"https://second.example.org\"}\n";
    let (rows, rejected) = parse_queue(text);
    check!(eq; rejected, 1);
    check!(eq; rows.len(), 2);
    check!(eq; rows.first().map(|row| row.name.as_str()), Some("Rock Bridge High School"));
    Ok(())
}
