use super::{analyse, Rules, Signals};
use census_domain::model::CoachRole;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn row(id: &str, role: &str, sport: &str, email: &str) -> String {
    format!("<tr class='sidearm-staff-member' data-member-id='{id}' data-category-id='xc'>\
        <td headers='col-fullname'><a>Jane Doe</a></td>\
        <td headers='col-staff_title'>{role}</td>\
        <td headers='col-staff_custom_1'>{sport}</td>\
        <td headers='col-staff_custom_2'>Varsity</td>\
        <td headers='col-staff_email'><script>var firstHalf = '{email}'; var secondHalf = 'school.org';</script></td></tr>")
}

fn page(rows: &[String]) -> String {
    format!(
        "<h1>Example High School</h1><article class='sidearm-staff'><table>{}</table></article>",
        rows.concat()
    )
}

#[test]
fn staff_candidates_keep_only_same_row_professional_contacts() -> TestResult {
    let html = page(&[
        row(
            "coach",
            "Head Varsity Coach",
            "Girls Cross Country",
            "coach",
        ),
        row(
            "student",
            "Student Head Coach",
            "Girls Cross Country",
            "student",
        ),
        row(
            "trainer",
            "Athletic Trainer",
            "Girls Cross Country",
            "trainer",
        ),
        row("football", "Head Coach", "Football", "football"),
        row(
            "volleyball",
            "Head Coach",
            "Indoor Volleyball",
            "volleyball",
        ),
        row("ad", "Athletic Director", "Administration", "ad"),
        row("missing", "", "Girls Cross Country", "missing"),
    ]);
    let mut signals = Signals::default();
    analyse(
        &Rules::new()?,
        "https://school.org/staff",
        &html,
        &mut signals,
    )?;
    let actual: Vec<_> = signals
        .staff_contacts
        .iter()
        .map(|hit| (hit.member_id.as_str(), hit.email.as_str(), hit.role))
        .collect();
    check!(eq; actual, vec![
        ("coach", "coach@school.org", CoachRole::HeadCoach),
        ("ad", "ad@school.org", CoachRole::AthleticDirector),
    ]);
    check!(eq; signals.staff_pages, vec!["https://school.org/staff"]);
    check!(signals
        .staff_contacts
        .iter()
        .all(|hit| hit.published_school == "Example High School"
            && hit.url == "https://school.org/staff"
            && hit.name == "Jane Doe"));
    check!(signals.emails.is_empty());
    Ok(())
}

#[test]
fn unsupported_pages_do_not_claim_staff_inspection() -> TestResult {
    let html = "<p>Head Coach Jane Doe <a href='mailto:student@school.org'>Student</a></p>";
    let mut signals = Signals::default();
    analyse(
        &Rules::new()?,
        "https://school.org/team",
        html,
        &mut signals,
    )?;
    check!(signals.staff_contacts.is_empty());
    check!(signals.staff_pages.is_empty());
    Ok(())
}

#[test]
fn recognized_empty_staff_directory_retains_inspected_page() -> TestResult {
    let html = page(&[row(
        "student",
        "Student Athlete",
        "Cross Country",
        "student",
    )]);
    let mut signals = Signals::default();
    analyse(
        &Rules::new()?,
        "https://school.org/staff",
        &html,
        &mut signals,
    )?;
    check!(signals.staff_contacts.is_empty());
    check!(eq; signals.staff_pages, vec!["https://school.org/staff"]);
    Ok(())
}

#[test]
fn staff_candidate_budget_refuses_before_signal_mutation() -> TestResult {
    let html = page(&[row("coach", "Head Coach", "Cross Country", "coach")]);
    let rules = Rules::new()?;
    let mut signals = Signals::default();
    analyse(&rules, "https://school.org/staff", &html, &mut signals)?;
    let hit = signals
        .staff_contacts
        .first()
        .ok_or("staff candidate")?
        .clone();
    signals.staff_contacts = vec![hit; 16_384];
    let previous = signals.clone();
    check!(matches!(
        analyse(&rules, "https://school.org/staff", &html, &mut signals),
        Err(crate::CrawlError::Resource {
            requested: 16_385,
            limit: 16_384,
            ..
        })
    ));
    check!(eq; signals, previous);
    Ok(())
}

#[test]
fn script_text_cannot_supply_staff_role_name_or_sport() -> TestResult {
    let rules = Rules::new()?;
    for html in [
        page(&[row(
            "role",
            "<script>var example='Head Coach';</script>",
            "Cross Country",
            "student",
        )]),
        page(&[row(
            "sport",
            "Head Coach",
            "<script>var example='Cross Country';</script>",
            "student",
        )]),
        page(&[row("name", "Head Coach", "Cross Country", "student")]).replace(
            "<a>Jane Doe</a>",
            "<a><script>var example='Jane Doe';</script></a>",
        ),
    ] {
        let mut signals = Signals::default();
        analyse(&rules, "https://school.org/staff", &html, &mut signals)?;
        check!(signals.staff_contacts.is_empty());
    }
    Ok(())
}

#[test]
fn quoted_attribute_cannot_supply_a_coaching_role() -> TestResult {
    let html = page(&[row("manager", "Manager", "Cross Country", "manager")]).replace(
        "headers='col-staff_title'",
        "headers='col-staff_title' title='>Head Coach'",
    );
    let mut signals = Signals::default();
    analyse(
        &Rules::new()?,
        "https://school.org/staff",
        &html,
        &mut signals,
    )?;
    check!(signals.staff_contacts.is_empty());
    Ok(())
}

#[test]
fn staff_page_budget_is_checked_before_append() -> TestResult {
    let html = page(&[]);
    let rules = Rules::new()?;
    let mut signals = Signals {
        staff_pages: vec!["https://school.org/staff".to_owned(); 12],
        ..Signals::default()
    };
    analyse(&rules, "https://school.org/staff", &html, &mut signals)?;
    check!(eq; signals.staff_pages.len(), 13);
    let previous = signals.clone();
    check!(matches!(
        analyse(&rules, "https://school.org/staff", &html, &mut signals),
        Err(crate::CrawlError::Resource {
            requested: 14,
            limit: 13,
            ..
        })
    ));
    check!(eq; signals, previous);
    Ok(())
}

#[test]
fn staff_sports_reuse_the_shared_contact_classifier() -> TestResult {
    let html = page(&[
        row("xc", "Head Coach", "Cross-Country", "xc"),
        row("track", "Head Coach", "Track", "track"),
        row("indoor", "Head Coach", "Indoor Track", "indoor"),
        row(
            "volleyball",
            "Head Coach",
            "Indoor Volleyball",
            "volleyball",
        ),
    ]);
    let mut signals = Signals::default();
    analyse(
        &Rules::new()?,
        "https://school.org/staff",
        &html,
        &mut signals,
    )?;
    let actual: Vec<_> = signals
        .staff_contacts
        .iter()
        .map(|hit| (hit.member_id.as_str(), hit.sport))
        .collect();
    check!(eq; actual, vec![
        ("xc", Some(census_domain::model::Sport::CrossCountry)),
        ("track", Some(census_domain::model::Sport::OutdoorTrack)),
        ("indoor", Some(census_domain::model::Sport::IndoorTrack)),
    ]);
    check!(eq; crate::coach_contacts::parse_sport("Indoor Volleyball"), None);
    Ok(())
}

fn profile(name: &str, role: &str, mailbox: &str) -> String {
    format!(
        "<ul class='profile-list-box'><li>{name}</li><li>{role}</li>\
        <li><a href='mailto:{mailbox}'>{mailbox}</a></li></ul>"
    )
}

#[test]
fn generic_profile_binds_director_mailbox_without_inventing_team_or_owner() -> TestResult {
    let mut signals = Signals::default();
    let url = "https://lotusschool.org/athletics-dpt/";
    analyse(
        &Rules::new()?,
        url,
        &profile(
            "Deon Jackson",
            "Athletic Director",
            "djackson@lotusschool.org",
        ),
        &mut signals,
    )?;
    check!(eq; signals.staff_contacts.len(), 1);
    let hit = signals.staff_contacts.first().ok_or("director candidate")?;
    check!(eq; hit.name, "Deon Jackson");
    check!(eq; hit.published_role, "Athletic Director");
    check!(eq; hit.email, "djackson@lotusschool.org");
    check!(eq; hit.role, CoachRole::AthleticDirector);
    check!(eq; hit.url, url);
    check!(hit.sport.is_none());
    check!(eq; hit.gender, census_domain::model::Gender::Unknown);
    check!(
        hit.published_school.is_empty()
            && hit.member_id.is_empty()
            && hit.category_id.is_empty()
            && hit.published_sport.is_empty()
            && hit.published_level.is_empty()
    );
    check!(signals.school_mailboxes.is_empty() && signals.emails.is_empty());
    check!(eq; signals.staff_pages, vec![url]);
    Ok(())
}

#[test]
fn generic_profiles_do_not_borrow_adjacent_student_mailboxes() -> TestResult {
    let html = format!(
        "{}{}{}",
        profile("Deon Jackson", "Athletic Director", "djackson@school.org"),
        profile(
            "Jane Student",
            "Student Athletic Director",
            "student@school.org"
        ),
        profile("Alex Player", "Head Coach Football", "player@school.org")
    );
    let mut signals = Signals::default();
    analyse(
        &Rules::new()?,
        "https://school.org/staff",
        &html,
        &mut signals,
    )?;
    check!(eq; signals.staff_contacts.len(), 1);
    check!(eq; signals.staff_contacts.first().map(|hit| hit.email.as_str()), Some("djackson@school.org"));
    Ok(())
}

#[test]
fn generic_profiles_reject_ambiguous_nested_and_inert_contacts() -> TestResult {
    let valid = profile("Deon Jackson", "Athletic Director", "director@school.org");
    let cases = [
        valid.replace("Athletic Director", "Student Athletic Director"),
        valid.replace("Athletic Director", "Not Athletic Director"),
        valid.replace("Athletic Director", "Contact the Athletic Director"),
        valid.replace("Deon Jackson", "Jane Doe (Student)"),
        valid.replace("Deon Jackson", "Jane Doe Student"),
        valid.replace("<li><a", "<li>Student contact Jane Doe <a"),
        valid.replace(">director@school.org</a>", ">student@school.org</a>"),
        valid.replace("Athletic Director", ""),
        valid.replace(
            "</a>",
            "</a><a href='mailto:student@school.org'>Student</a>",
        ),
        valid.replace("Deon Jackson", "<div>Deon Jackson</div>"),
        valid.replace("Athletic Director", "<script>Athletic Director</script>"),
        valid.replace(
            "Athletic Director",
            "Athletic<!-- gap --> Director<li>Other Person</li>",
        ),
        format!("<template>{valid}</template>"),
        format!("<script>{valid}</script>"),
        format!("<!-- {valid} -->"),
        valid.replace("profile-list-box", "student-profile"),
    ];
    for html in cases {
        let mut signals = Signals::default();
        let result = analyse(
            &Rules::new()?,
            "https://school.org/staff",
            &html,
            &mut signals,
        );
        check!(signals.staff_contacts.is_empty());
        if result.is_ok() {
            check!(signals.school_mailboxes.is_empty());
        }
    }
    Ok(())
}

#[test]
fn generic_profile_budget_is_not_silent_empty() -> TestResult {
    let html = profile("Deon Jackson", "Athletic Director", "director@school.org").repeat(321);
    let mut signals = Signals::default();
    check!(analyse(
        &Rules::new()?,
        "https://school.org/staff",
        &html,
        &mut signals
    )
    .is_err());
    check!(signals.staff_contacts.is_empty());
    Ok(())
}
