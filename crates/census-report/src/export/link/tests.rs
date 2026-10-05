use super::*;
use census_domain::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn school() -> TestResult<CanonicalSchool> {
    let (school, _) = CanonicalSchool::new(
        UsJurisdiction::Tennessee,
        "Page High School",
        census_domain::model::normalize_name("Page High School"),
    );
    Ok(school)
}

fn association(association: &str, id: &str) -> SourceIdentity {
    SourceIdentity::new(SourceNamespace::association_school(association), id)
}

fn lane(association: &str, url: &str, note: &str) -> Evidence {
    let mut evidence = Evidence::parsed(
        census_domain::model::SourceRef::new(association, Some(url.to_string())),
        "2026-09-22",
    );
    evidence.note = Some(note.to_string());
    evidence
}

#[test]
fn an_association_link_carries_its_lane_provenance() -> TestResult {
    let mut school = school()?;
    school.source_identities.push(
        association("tssaa", "157")
            .with_url("https://portal.tssaa.org/common/directory/?id=157".to_string()),
    );
    school.evidence.push(lane(
        "tssaa",
        "https://portal.tssaa.org/common/directory/?id=157",
        "association:tssaa lane crates/census-crawl/tests/fixtures/tssaa/directory_id157.html sha256=2f920f31115b5e96e2a1155747022e96baf54d41069c52a16701a3fb814a6453 generation 3b177e8b31e016783339849762ef0467c652abc0c58e2095aecd3375a5becd0d",
    ));
    let fields = link_fields(&school);
    check!(eq; fields[0].as_str(), school.id.as_str());
    check!(eq; fields[1].as_str(), "association_school:tssaa");
    check!(eq; fields[2].as_str(), "157");
    check!(eq; fields[3].as_str(), "https://portal.tssaa.org/common/directory/?id=157");
    check!(eq; fields[4].as_str(), "tssaa");
    check!(eq; fields[5].as_str(), "https://portal.tssaa.org/common/directory/?id=157");
    check!(eq; fields[6].as_str(), "2026-09-22");
    check!(eq; fields[7].contains("sha256=2f920f31115b5e96"), true);
    Ok(())
}

#[test]
fn evidence_pairs_by_source_when_the_identity_has_no_url() -> TestResult {
    let mut school = school()?;
    school.source_identities.push(association("tssaa", "157"));
    school.evidence.push(lane(
        "tssaa",
        "https://portal.tssaa.org/common/directory/?id=157",
        "lane capture",
    ));
    school.evidence.push(Evidence::fetched(
        census_domain::model::SourceRef::id("nces-ccd"),
        "2026-10-01",
    ));
    let fields = link_fields(&school);
    check!(eq; fields[3].as_str(), "");
    check!(eq; fields[4].as_str(), "tssaa");
    check!(eq; fields[6].as_str(), "2026-09-22");
    check!(eq; fields[7].as_str(), "lane capture");
    Ok(())
}

#[test]
fn a_link_without_published_evidence_keeps_the_identity_only() -> TestResult {
    let mut school = school()?;
    school
        .source_identities
        .push(association("tssaa", "157").with_url("https://tssaa.test/157".to_string()));
    school.evidence.push(Evidence::fetched(
        census_domain::model::SourceRef::id("nces-ccd"),
        "2026-10-01",
    ));
    let fields = link_fields(&school);
    check!(eq; fields[2].as_str(), "157");
    check!(eq; fields[3].as_str(), "https://tssaa.test/157");
    check!(eq; fields[4].as_str(), "");
    check!(eq; fields[5].as_str(), "");
    check!(eq; fields[6].as_str(), "");
    check!(eq; fields[7].as_str(), "");
    Ok(())
}

#[test]
fn a_school_without_an_association_identity_emits_empty_fields() -> TestResult {
    let mut school = school()?;
    school.source_identities.push(SourceIdentity::new(
        SourceNamespace::MilesplitSchool,
        "page-high",
    ));
    let fields = link_fields(&school);
    let empty: [String; 8] = Default::default();
    check!(eq; fields, empty);
    Ok(())
}

#[test]
fn the_first_identity_wins_in_namespace_and_id_order() -> TestResult {
    let mut school = school()?;
    school.source_identities.push(association("tssaa", "z-9"));
    school.source_identities.push(association("tssaa", "a-1"));
    school
        .source_identities
        .push(association("nchsaa", "ZCUM49"));
    let fields = link_fields(&school);
    check!(eq; fields[1].as_str(), "association_school:nchsaa");
    check!(eq; fields[2].as_str(), "ZCUM49");
    Ok(())
}

#[test]
fn a_campus_pair_publishes_its_own_link_key() -> TestResult {
    let mut main = school()?;
    main.source_identities.push(association("tssaa", "157"));
    main.evidence.push(lane(
        "tssaa",
        "https://portal.tssaa.org/common/directory/?id=157",
        "main campus lane",
    ));
    let (mut east, _) = CanonicalSchool::new(
        UsJurisdiction::Tennessee,
        "Page High School (East)",
        census_domain::model::normalize_name("Page High School (East)"),
    );
    east.source_identities.push(association("tssaa", "158"));
    east.evidence.push(lane(
        "tssaa",
        "https://portal.tssaa.org/common/directory/?id=158",
        "east campus lane",
    ));
    let main_fields = link_fields(&main);
    let east_fields = link_fields(&east);
    check!(ne; main_fields[0], east_fields[0]);
    check!(eq; main_fields[2].as_str(), "157");
    check!(eq; east_fields[1].as_str(), "association_school:tssaa");
    check!(eq; east_fields[2].as_str(), "158");
    Ok(())
}
