use super::*;
use crate::sources::AdapterContext;
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Gender, SourceNamespace, Sport,
};
use census_domain::UsJurisdiction;
use census_store::Table;
use serde_json::json;
use std::collections::HashSet;

const LISTING: &str = include_str!("../../../tests/fixtures/mshsl/schools_listing.html");
const AITKIN: &str =
    include_str!("../../../tests/fixtures/mshsl/school_detail_aitkin-high-school.html");
const FOLEY: &str =
    include_str!("../../../tests/fixtures/mshsl/school_detail_foley-high-school.html");
const WAYZATA: &str =
    include_str!("../../../tests/fixtures/mshsl/school_detail_wayzata-high-school.html");
const ACADEMIC_ARTS: &str =
    include_str!("../../../tests/fixtures/mshsl/school_detail_academic-arts-high-school.html");
// Live captures (2026-09-20, HTTP 200) of
// https://www.mshsl.org/jsonapi/views/teams/list_school?views-argument[]=611 (sparse fieldset) and
// https://www.mshsl.org/api/coaches/589034, verbatim.
const WAYZATA_TEAMS: &str = include_str!("../../../tests/fixtures/mshsl/team_nodes_wayzata.json");
const WAYZATA_TF_COACHES: &str =
    include_str!("../../../tests/fixtures/mshsl/coach_records_wayzata_track_boys.json");
// Live captures (2026-09-20T14:39Z, HTTP 200, verbatim) of school 7's team list and of the two
// coach payloads its two track nodes point at, plus the page-0 listing with the pager trimmed so
// the cached run below stays on one page.
const LISTING_FIRST_PAGE: &str =
    include_str!("../../../tests/fixtures/mshsl/schools_listing_first_page.html");
const AITKIN_TEAMS: &str = include_str!("../../../tests/fixtures/mshsl/team_nodes_aitkin.json");
const AITKIN_TF_BOYS: &str =
    include_str!("../../../tests/fixtures/mshsl/coach_records_aitkin_track-and-field-boys.json");
const AITKIN_TF_GIRLS: &str =
    include_str!("../../../tests/fixtures/mshsl/coach_records_aitkin_track-and-field-girls.json");

const OBSERVED_ON: &str = "2026-09-20";

/// Seed the fetcher's on-disk cache for `url` under the key `Fetcher` derives
/// (`sha256(method \x1f url \x1f body)[..16]`), so `collect` can be driven end to end with no socket.
fn seed_cache(cache_dir: &std::path::Path, url: &str, body: &str) {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"GET");
    hasher.update([0x1f]);
    hasher.update(url.as_bytes());
    hasher.update([0x1f]);
    let key: String = hasher.finalize()[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let meta = json!({
        "url": url,
        "method": "GET",
        "status": 200,
        "sha256": format!("{:x}", Sha256::digest(body.as_bytes())),
        "bytes": body.len(),
        "fetched_at": "2026-09-20T14:39:00Z",
    });
    std::fs::create_dir_all(cache_dir).expect("cache dir");
    std::fs::write(cache_dir.join(format!("{key}.body")), body).expect("cache body");
    std::fs::write(
        cache_dir.join(format!("{key}.meta.json")),
        serde_json::to_vec(&meta).expect("cache meta"),
    )
    .expect("cache meta written");
}

fn listing_row<'a>(rows: &'a [SchoolListRow], slug: &str) -> &'a SchoolListRow {
    rows.iter()
        .find(|row| row.slug == slug)
        .expect("fixture row")
}

/// A listing row for a school whose listing page was not captured: the school's own page supplies the
/// name and the slug is the real one (page 0 of the capture stops at Aitkin, so later schools are
/// reached on their own pages).
fn detail_row(slug: &str, detail: &SchoolDetail) -> SchoolListRow {
    SchoolListRow {
        slug: slug.to_string(),
        name: detail.name.clone().unwrap_or_default(),
        city: None,
    }
}

/// The listing row the crawl would have seen: the captured one when page 0 holds the school, else a
/// row built from the school's own page.
fn row_for(rows: &[SchoolListRow], slug: &str, detail: &SchoolDetail) -> SchoolListRow {
    rows.iter()
        .find(|row| row.slug == slug)
        .cloned()
        .unwrap_or_else(|| detail_row(slug, detail))
}

#[test]
fn listing_rows_carry_slug_name_and_city() {
    let rows = parse_school_list(LISTING);
    assert_eq!(rows.len(), 8, "listing fixture holds eight school rows");
    let first = &rows[0];
    assert_eq!(first.slug, "aasen-home-school");
    assert_eq!(first.name, "Aasen Home School");
    assert_eq!(first.city.as_deref(), Some("Clearwater"));
    let aitkin = listing_row(&rows, "aitkin-high-school");
    assert_eq!(aitkin.name, "Aitkin High School");
    assert_eq!(aitkin.city.as_deref(), Some("Aitkin"));
    assert!(rows
        .iter()
        .all(|row| !row.slug.contains('/') && !row.name.is_empty()));
}

#[test]
fn pagination_follows_the_real_pager() {
    // The capture is listing page 0 and shows numbered links 0..8 plus a "next page" anchor.
    assert_eq!(parse_next_listing_page(LISTING, 0), Some(1));
    assert_eq!(parse_next_listing_page(LISTING, 1), Some(2));
    assert_eq!(
        parse_next_listing_page(LISTING, 8),
        None,
        "the capture has no link to page 9"
    );
    assert_eq!(parse_next_listing_page(LISTING, 13), None);
    assert_eq!(
        parse_next_listing_page(FOLEY, 0),
        None,
        "a detail page has no pager"
    );
    // A pager whose only continuation is the "next" anchor (attributes reordered) is still followed.
    let next_only =
        r#"<nav class="pager"><a rel="next" class="pager__link" href="?page=9">Next ›</a></nav>"#;
    assert_eq!(parse_next_listing_page(next_only, 8), Some(9));
    assert_eq!(parse_next_listing_page(next_only, 7), None);
    assert_eq!(parse_next_listing_page("", 0), None);
    assert_eq!(listing_page_url(0), "https://www.mshsl.org/schools");
    assert_eq!(listing_page_url(3), "https://www.mshsl.org/schools?page=3");
    assert_eq!(
        school_page_url("foley-high-school"),
        "https://www.mshsl.org/schools/foley-high-school"
    );
}

#[test]
fn cfemail_decodes_to_the_published_addresses() {
    // Real Cloudflare payloads from the three captures.
    assert_eq!(
        decode_cfemail("8de0eff8e8eee6e8fffecdecfdfdfea3e4fee9b8bca3e2ffea").as_deref(),
        Some("mbueckers@apps.isd51.org")
    );
    assert_eq!(
        decode_cfemail("6e04060b001c070d051d01002e071d0a5f40011c09").as_deref(),
        Some("jhenrickson@isd1.org")
    );
    assert_eq!(
        decode_cfemail("254840424d444b0b554a515140576552445c5f44514456464d4a4a49560b4a5742")
            .as_deref(),
        Some("meghan.potter@wayzataschools.org")
    );
    // The attribute form a DOM snapshot carries decodes with the same routine.
    let attributes = decode_cfemail_fragment(
        r#"<span data-cfemail="8de0eff8e8eee6e8fffecdecfdfdfea3e4fee9b8bca3e2ffea"></span>"#,
    );
    assert_eq!(attributes, vec!["mbueckers@apps.isd51.org".to_string()]);
    for malformed in ["", "abc", "zz00", "00", "no-hex-here", "  "] {
        assert_eq!(
            decode_cfemail(malformed),
            None,
            "{malformed:?} must not decode"
        );
    }
    assert!(decode_cfemail_fragment("<p>no addresses here</p>").is_empty());
}

#[test]
fn school_page_yields_canonical_school_with_identity_and_evidence() {
    let rows = parse_school_list(LISTING);
    let detail = parse_school_detail(AITKIN);
    assert_eq!(detail.name.as_deref(), Some("Aitkin High School"));
    assert_eq!(detail.school_id.as_deref(), Some("7"));
    assert_eq!(detail.enrollment, Some(291));
    assert_eq!(
        detail.website.as_deref(),
        Some("https://isd1.rschoolteams.com/")
    );
    let row = listing_row(&rows, "aitkin-high-school");
    let url = school_page_url(&row.slug);
    let (school, school_id) = school_entities(row, &detail, &url, OBSERVED_ON).expect("school");
    assert_eq!(school.name, "Aitkin High School");
    assert_eq!(school.state, Some(UsJurisdiction::Minnesota));
    assert_eq!(school.city.as_deref(), Some("Aitkin"));
    assert_eq!(school.association.as_deref(), Some("mshsl"));
    assert_eq!(school.enrollment, Some(291));
    assert_eq!(
        school.school_website.as_deref(),
        Some("https://isd1.rschoolteams.com/")
    );
    assert_eq!(
        school.id,
        CanonicalSchool::mint(
            UsJurisdiction::Minnesota,
            "Aitkin High School",
            &normalize_name("Aitkin High School")
        )
    );
    assert_eq!(school_id, school.id);
    let identity = school.source_identities.first().expect("identity");
    assert_eq!(
        identity.namespace,
        SourceNamespace::AssociationSchool {
            association: "mshsl".to_string()
        }
    );
    assert_eq!(identity.id, "7");
    assert_eq!(identity.url.as_deref(), Some(url.as_str()));
    let evidence = school.evidence.first().expect("evidence");
    assert_eq!(evidence.source.url.as_deref(), Some(url.as_str()));
    assert_eq!(evidence.observed_on, OBSERVED_ON);
    // Neither the listing row nor the page contributes a name: no school can be minted.
    let nameless = SchoolListRow {
        slug: "nameless".to_string(),
        name: String::new(),
        city: None,
    };
    assert!(school_entities(&nameless, &SchoolDetail::default(), &url, OBSERVED_ON).is_none());
    // A listing row name is enough: the school is minted from the row even with no page facts.
    let (from_row, _) =
        school_entities(row, &SchoolDetail::default(), &url, OBSERVED_ON).expect("school");
    assert_eq!(from_row.name, "Aitkin High School");
    assert!(from_row.enrollment.is_none());
}

#[test]
fn athletic_directors_are_the_only_admin_rows_emitted() {
    let cases = [
        (
            AITKIN,
            "aitkin-high-school",
            vec!["jhenrickson@isd1.org", "ahills@isd1.org"],
        ),
        (
            FOLEY,
            "foley-high-school",
            vec!["mbueckers@apps.isd51.org", ""],
        ),
        (
            ACADEMIC_ARTS,
            "academic-arts-high-school",
            vec!["hannah.couch@academicarts.org"],
        ),
    ];
    let rows = parse_school_list(LISTING);
    for (html, slug, emails) in cases {
        let detail = parse_school_detail(html);
        let row = row_for(&rows, slug, &detail);
        let (_, school_id) =
            school_entities(&row, &detail, &school_page_url(slug), OBSERVED_ON).expect("school");
        let coaches = ad_coaches(
            &detail,
            &school_id,
            &provider_key(&row, &detail),
            &school_page_url(slug),
            OBSERVED_ON,
        );
        assert_eq!(coaches.len(), emails.len(), "{slug}: one row per AD entry");
        for (coach, email) in coaches.iter().zip(emails) {
            assert_eq!(coach.role, CoachRole::AthleticDirector, "{slug}");
            assert_eq!(coach.sport, None, "athletic directors are school-wide");
            assert_eq!(coach.gender, Gender::Mixed);
            assert_eq!(
                coach.professional_email.as_deref().unwrap_or(""),
                email,
                "{slug}"
            );
            assert_eq!(coach.evidence.len(), 1);
            assert_eq!(coach.source_identities.len(), 1);
        }
    }
    // Foley's assistant director is published without a contact: that row exists with no email.
    let foley = parse_school_detail(FOLEY);
    let assistant = foley
        .admin
        .iter()
        .find(|entry| entry.role.contains("Assistant"))
        .expect("assistant director entry");
    assert_eq!(assistant.name, "Alyssa Stewart");
    assert!(assistant.email().is_none());
}

#[test]
fn office_roles_never_become_coaches_or_athletic_directors() {
    // Wayzata publishes fifteen Administration entries, two of them "AD Administrative Assistant"
    // with addresses and an athletic trainer on a hospital domain.
    let detail = parse_school_detail(WAYZATA);
    assert_eq!(detail.admin.len(), 15);
    let foley = parse_school_detail(FOLEY);
    assert!(foley
        .admin
        .iter()
        .any(|entry| entry.role.contains("Administrative Assistant") && entry.email().is_some()));

    let rows = parse_school_list(LISTING);
    let row = row_for(&rows, "wayzata-high-school", &detail);
    let (school, school_id) =
        school_entities(&row, &detail, &school_page_url(&row.slug), OBSERVED_ON).expect("school");
    let coaches = ad_coaches(
        &detail,
        &school_id,
        "611",
        &school_page_url(&row.slug),
        OBSERVED_ON,
    );
    assert_eq!(coaches.len(), 2, "only the AD and the assistant AD");
    let emitted = serde_json::to_string(&(school, coaches)).expect("json");
    for office in [
        "chris.easton@wayzataschools.org",
        "kari.rohrich@wayzataschools.org",
        "chris.thein@wayzataschools.org",
        "Chris Easton",
        "Kari Rohrich",
        "Scott Gengler",
        "Robb Virgin",
        "Donald Krubsack",
    ] {
        assert!(!emitted.contains(office), "{office} must not be emitted");
    }
    assert!(emitted.contains("meghan.potter@wayzataschools.org"));
    assert!(emitted.contains("sydney.helmbrecht@wayzataschools.org"));
    // Foley publishes its AD administrative assistant with an address in the same block.
    let foley = parse_school_detail(FOLEY);
    let foley_row = row_for(&rows, "foley-high-school", &foley);
    let foley_url = school_page_url(&foley_row.slug);
    let (foley_school, foley_id) =
        school_entities(&foley_row, &foley, &foley_url, OBSERVED_ON).expect("school");
    let foley_emitted = serde_json::to_string(&(
        foley_school,
        ad_coaches(&foley, &foley_id, "175", &foley_url, OBSERVED_ON),
    ))
    .expect("json");
    for office in [
        "cogross@apps.isd51.org",
        "Corri Gross",
        "Joel Foss",
        "Daniel Posthumus",
    ] {
        assert!(
            !foley_emitted.contains(office),
            "{office} must not be emitted"
        );
    }

    for label in [
        "AD Administrative Assistant",
        "Activities Director Secretary",
        "Principal",
        "Superintendent",
        "Athletic Trainer/Medical",
        "Band Director",
        "Choir Director",
        "Orchestra Director",
        "Yearbook Advisor",
        "Newspaper Advisor",
        "Title IX Officer",
        "Boys Sports Representative",
        "Girls Sports Representative",
        "Business Manager",
    ] {
        assert_eq!(ad_role(label), None, "{label} is not an AD role");
    }
    assert_eq!(
        ad_role("Activities Director:"),
        Some(CoachRole::AthleticDirector)
    );
    assert_eq!(
        ad_role("Assistant Activities Director"),
        Some(CoachRole::AthleticDirector)
    );
}

#[test]
fn published_phones_and_consumer_mailboxes_never_reach_the_store() {
    let detail = parse_school_detail(WAYZATA);
    let domains = school_domains(&detail);
    assert_eq!(
        domains,
        vec![
            "wayzataschools.org".to_string(),
            "wayzatatrojans.org".to_string()
        ]
    );
    let nodes = select_team_nodes(&parse_team_nodes(WAYZATA_TEAMS));
    let team = nodes
        .iter()
        .find(|node| node.alias.contains("track-and-field-boys"))
        .expect("track and field node");
    let html_url = school_page_url("wayzata-high-school");
    let rows = parse_school_list(LISTING);
    let row = row_for(&rows, "wayzata-high-school", &detail);
    let (school, school_id) =
        school_entities(&row, &detail, &html_url, OBSERVED_ON).expect("school");
    let teams = vec![TeamCoaches {
        node: team.clone(),
        api_url: format!("{COACH_API_PREFIX}{}", team.nid),
        records: parse_coach_records(WAYZATA_TF_COACHES),
    }];
    let coaches = coach_entities(&teams, &school_id, &domains, OBSERVED_ON);
    assert_eq!(
        coaches.len(),
        10,
        "ten real records, all of them MSHSL levels"
    );
    let serialized = serde_json::to_string(&(school, coaches.clone())).expect("json");
    for leak in [
        "763-745-6995",
        "763-745-6889",
        "612-387-2904",
        "6124239766",
        "giesen21@hotmail.com",
        "mike95asmith@gmail.com",
        "tel:",
        "field_work_phone",
    ] {
        assert!(!serialized.contains(leak), "{leak} must not be emitted");
    }
    assert!(coaches.iter().all(|coach| coach.phone.is_none()));
    let head = coaches
        .iter()
        .find(|coach| coach.role == CoachRole::HeadCoach)
        .expect("head coach");
    assert_eq!(head.name, "Aaron Berndt");
    assert_eq!(head.sport, Some(Sport::OutdoorTrack));
    assert_eq!(head.gender, Gender::Boys);
    assert_eq!(
        head.professional_email.as_deref(),
        Some("aaron.berndt@wayzataschools.org")
    );
    assert_eq!(
        head.evidence.len(),
        2,
        "the coach payload and the team page"
    );
}

#[test]
fn team_nodes_filter_to_track_and_cross_country() {
    let nodes = parse_team_nodes(WAYZATA_TEAMS);
    assert_eq!(nodes.len(), 39, "Wayzata publishes 39 team nodes");
    let selected = select_team_nodes(&nodes);
    let picks: Vec<(&str, &str)> = selected
        .iter()
        .map(|node| (node.nid.as_str(), node.alias.as_str()))
        .collect();
    assert_eq!(
        picks,
        vec![
            (
                "589008",
                "/schools/wayzata-high-school/cross-country-running-boys/2026"
            ),
            (
                "589009",
                "/schools/wayzata-high-school/cross-country-running-girls/2026"
            ),
            (
                "589034",
                "/schools/wayzata-high-school/track-and-field-boys/2027"
            ),
            (
                "589035",
                "/schools/wayzata-high-school/track-and-field-girls/2027"
            ),
        ]
    );
    assert_eq!(
        team_sport("/schools/x/cross-country-running-girls/2026"),
        Some((Sport::CrossCountry, Gender::Girls))
    );
    assert_eq!(
        team_sport("/schools/x/track-and-field-boys/2027"),
        Some((Sport::OutdoorTrack, Gender::Boys))
    );
    assert_eq!(team_sport("/schools/x/football/2026"), None);
    assert_eq!(team_sport(""), None);
    assert_eq!(
        selected.first().and_then(TeamNode::page_url).as_deref(),
        Some("https://www.mshsl.org/schools/wayzata-high-school/cross-country-running-boys/2026")
    );
}

#[test]
fn coach_levels_map_to_roles_and_only_school_domains_survive() {
    assert!(is_published_level("Head Coach"));
    assert!(!is_published_level("Non-MSHSL Coach"));
    assert!(!is_published_level("mshsl sub-coach"));
    assert_eq!(coach_role("Head Coach"), Some(CoachRole::HeadCoach));
    assert_eq!(
        coach_role("Assistant Coach"),
        Some(CoachRole::AssistantCoach)
    );
    assert_eq!(coach_role("Volunteer Coach"), None);
    assert_eq!(coach_role("Non-MSHSL Coach"), None);
    assert_eq!(coach_role("MSHSL Sub-Coach"), None);

    let domains = vec!["wayzataschools.org".to_string()];
    assert_eq!(
        accept_coach_email("mark.popp@wayzataschools.org", &domains).as_deref(),
        Some("mark.popp@wayzataschools.org")
    );
    assert_eq!(accept_coach_email("giesen21@hotmail.com", &domains), None);
    assert_eq!(
        accept_coach_email("coach@mail.wayzataschools.org", &domains).as_deref(),
        Some("coach@mail.wayzataschools.org")
    );
    assert_eq!(
        accept_coach_email("coach@other-district.org", &domains),
        None
    );
    assert_eq!(accept_coach_email("not-an-address", &domains), None);
    assert_eq!(
        accept_coach_email("aaron.berndt@wayzataschools.org", &[]),
        None,
        "no reference domain, no email"
    );

    // The unexercised branch: a record whose level is not published and a non-MSHSL level record.
    let node = TeamNode {
        nid: "1".to_string(),
        title: "Wayzata High School Track and Field, Boys".to_string(),
        alias: "/schools/wayzata-high-school/track-and-field-boys/2027".to_string(),
    };
    let teams = vec![TeamCoaches {
        node,
        api_url: format!("{COACH_API_PREFIX}1"),
        records: vec![
            CoachRecord {
                name: "Unpublished Coach".to_string(),
                level: "Non-MSHSL Coach".to_string(),
                email: Some("p@example.com".to_string()),
            },
            CoachRecord {
                name: "Sub Coach".to_string(),
                level: "MSHSL Sub-Coach".to_string(),
                email: Some("s@wayzataschools.org".to_string()),
            },
            CoachRecord {
                name: "Volunteer Coach".to_string(),
                level: "Volunteer Coach".to_string(),
                email: None,
            },
        ],
    }];
    let school_id = CanonicalSchool::mint(
        UsJurisdiction::Minnesota,
        "Wayzata High School",
        &normalize_name("Wayzata High School"),
    );
    assert!(coach_entities(&teams, &school_id, &domains, OBSERVED_ON).is_empty());
}

#[test]
fn ad_email_fill_rate_on_the_fixtures() {
    // Measured over the four school captures: AD rows and how many carry a decoded address.
    let mut rows_total = 0usize;
    let mut with_email = 0usize;
    let school_rows = parse_school_list(LISTING);
    for (html, slug) in [
        (AITKIN, "aitkin-high-school"),
        (FOLEY, "foley-high-school"),
        (WAYZATA, "wayzata-high-school"),
        (ACADEMIC_ARTS, "academic-arts-high-school"),
    ] {
        let detail = parse_school_detail(html);
        let row = row_for(&school_rows, slug, &detail);
        let (_, school_id) =
            school_entities(&row, &detail, &school_page_url(slug), OBSERVED_ON).expect("school");
        let coaches = ad_coaches(
            &detail,
            &school_id,
            "x",
            &school_page_url(slug),
            OBSERVED_ON,
        );
        rows_total += coaches.len();
        with_email += coaches
            .iter()
            .filter(|coach| coach.professional_email.is_some())
            .count();
    }
    assert_eq!(rows_total, 7, "seven AD rows across the four captures");
    assert_eq!(with_email, 6, "six of them publish an address (85.7%)");
}

#[test]
fn malformed_payloads_yield_zero_rows_instead_of_panicking() {
    assert!(parse_school_list("<html><body>no rows</body></html>").is_empty());
    assert!(parse_school_list("").is_empty());
    assert!(parse_next_listing_page("", 0).is_none());
    assert!(parse_next_listing_page("<a href=\"?page=abc\">", 0).is_none());
    assert!(parse_next_listing_page("<a href=\"/schools/foo\">next</a>", 0).is_none());
    assert!(parse_team_nodes("not json").is_empty());
    assert!(parse_team_nodes("{\"data\":[{\"attributes\":{\"title\":\"no nid\"}}]}").is_empty());
    assert!(parse_coach_records("{\"error\":\"not an array\"}").is_empty());
    assert!(parse_coach_records("").is_empty());
    assert!(parse_admin_entries("<html></html>").is_empty());
    assert!(decode_cfemail_fragment("").is_empty());
    let empty = parse_school_detail("");
    assert_eq!(empty, SchoolDetail::default());
    assert!(ad_coaches(
        &empty,
        &CanonicalSchool::mint(UsJurisdiction::Minnesota, "Empty", "empty"),
        "x",
        "u",
        OBSERVED_ON
    )
    .is_empty());
    assert!(school_domains(&empty).is_empty());
    assert!(select_team_nodes(&[]).is_empty());
}

#[test]
fn honorifics_are_stripped_before_minting() {
    assert_eq!(strip_honorific("Mr. Barry Mink"), "Barry Mink");
    assert_eq!(strip_honorific("Coach Jane Doe"), "Jane Doe");
    assert_eq!(strip_honorific("Dr. A. Smith"), "A. Smith");
    assert_eq!(strip_honorific("Matthew Bueckers"), "Matthew Bueckers");
    assert_eq!(strip_honorific("   "), "");
}

#[tokio::test]
async fn collect_fetches_parses_appends_journals_and_reports_from_a_warm_cache() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cache = dir.path().join("http");
    let teams_url = format!(
        "{TEAMS_VIEW_URL}?views-argument%5B%5D=7&fields%5Bnode--participant%5D=title,path,drupal_internal__nid"
    );
    seed_cache(&cache, &listing_page_url(0), LISTING_FIRST_PAGE);
    seed_cache(&cache, &school_page_url("aitkin-high-school"), AITKIN);
    seed_cache(&cache, &teams_url, AITKIN_TEAMS);
    seed_cache(&cache, &format!("{COACH_API_PREFIX}593477"), AITKIN_TF_BOYS);
    seed_cache(
        &cache,
        &format!("{COACH_API_PREFIX}593478"),
        AITKIN_TF_GIRLS,
    );

    let store = census_store::Store::open(dir.path().join("store")).expect("store");
    let fetcher = crate::net::Fetcher::new(
        &cache,
        None,
        std::time::Duration::from_millis(1),
        std::collections::HashMap::new(),
        Vec::new(),
    )
    .expect("fetcher");
    let ctx = AdapterContext {
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: census_domain::model::SchoolYear::new(2026).expect("2026 is a season"),
        observed_on: OBSERVED_ON.to_string(),
    };
    let options = Options {
        limit: Some(1),
        refresh: false,
        observed_on: OBSERVED_ON.to_string(),
        states: Vec::new(),
        school_names: vec!["Aitkin High School".to_string()],
    };
    let report = collect(&ctx, &options)
        .await
        .expect("collect returns a report");

    assert_eq!(
        report.rows, 1,
        "one school processed, the other seven rows filtered out"
    );
    assert_eq!(report.errors, 0);
    assert_eq!(
        report.requests, 0,
        "every response came from the seeded cache"
    );
    assert_eq!(
        report.from_cache, 5,
        "listing page, school page, team list and the two track coach payloads"
    );
    assert_eq!(
        report.with_email, 4,
        "two ADs and the two head coaches carry an address"
    );
    assert!(
        report
            .notes
            .iter()
            .any(|note| note.contains("office roles") && note.contains("not emitted")),
        "the report states that office roles were parsed and rejected: {:?}",
        report.notes
    );

    let schools = store
        .scan::<CanonicalSchool>(Table::Schools)
        .expect("schools log");
    assert_eq!(schools.len(), 1);
    assert_eq!(schools[0].name, "Aitkin High School");
    assert_eq!(schools[0].state, Some(UsJurisdiction::Minnesota));
    assert_eq!(schools[0].city.as_deref(), Some("Aitkin"));
    assert_eq!(schools[0].association.as_deref(), Some("mshsl"));
    assert_eq!(schools[0].enrollment, Some(291));

    let coaches = store
        .scan::<CanonicalCoach>(Table::Coaches)
        .expect("coaches log");
    assert_eq!(
        coaches.len(),
        4,
        "two AD rows plus one head coach per track team"
    );
    let emails: Vec<&str> = coaches
        .iter()
        .filter_map(|coach| coach.professional_email.as_deref())
        .collect();
    assert!(emails.contains(&"jhenrickson@isd1.org"));
    assert!(emails.contains(&"ahills@isd1.org"));
    assert!(emails.contains(&"acarlson@isd1.org"));
    assert!(emails.contains(&"avacarlson@isd1.org"));
    assert!(
        !emails
            .iter()
            .any(|address| address.contains("jforbord") || address.contains("jlong")),
        "the Non-MSHSL Coach records on the same payloads are dropped: {emails:?}"
    );
    let ads: Vec<&CanonicalCoach> = coaches
        .iter()
        .filter(|coach| coach.role == CoachRole::AthleticDirector)
        .collect();
    assert_eq!(ads.len(), 2);
    assert!(ads
        .iter()
        .all(|coach| coach.sport.is_none() && coach.gender == Gender::Mixed));
    let heads: Vec<&CanonicalCoach> = coaches
        .iter()
        .filter(|coach| coach.role == CoachRole::HeadCoach)
        .collect();
    assert_eq!(heads.len(), 2);
    assert!(heads
        .iter()
        .all(|coach| coach.sport == Some(Sport::OutdoorTrack)));
    assert!(heads
        .iter()
        .any(|coach| coach.name == "Adam Carlson" && coach.gender == Gender::Boys));
    assert!(heads
        .iter()
        .any(|coach| coach.name == "Ava Carlson" && coach.gender == Gender::Girls));
    let stored = serde_json::to_string(&coaches).expect("json");
    // The same Administration block publishes the principal, superintendent, advisors and trainer:
    // none of them may be stored as a coach or an athletic director.
    for office in [
        "Lisa DeMars",
        "Dan Stifter",
        "Taylor Meeks",
        "Jennifer Johnson",
        "Briana Tetrick",
        "Jason Henke",
        "Marc Carley",
    ] {
        assert!(
            !stored.contains(office),
            "{office} is an office role and must not be stored"
        );
    }
    assert!(
        coaches.iter().all(|coach| coach.phone.is_none()),
        "no phone column is parsed"
    );

    assert_eq!(
        store.journal_keys("mshsl_schools").expect("journal"),
        HashSet::from([String::from("MN:aitkin-high-school")])
    );
    assert_eq!(
        store.journal_keys("mshsl_coaches").expect("journal"),
        HashSet::from([String::from("MN:aitkin-high-school")])
    );

    // A re-run resumes: the journalled school is skipped and nothing is appended twice.
    let second = collect(&ctx, &options).await.expect("second collect");
    assert_eq!(second.rows, 0, "the school was already journalled");
    assert_eq!(second.requests, 0);
    assert_eq!(
        store
            .scan::<CanonicalSchool>(Table::Schools)
            .expect("schools log")
            .len(),
        1
    );
    assert_eq!(
        store
            .scan::<CanonicalCoach>(Table::Coaches)
            .expect("coaches log")
            .len(),
        4
    );
}
