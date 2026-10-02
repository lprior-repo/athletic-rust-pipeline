use super::*;
use census_domain::model::{Evidence, SourceIdentity, SourceNamespace, SourceRef};
use census_domain::school_directory::{CityName, PostalAddress, SourceLabel, StreetLine, ZipCode};
use census_domain::UsJurisdiction;
use sha2::{Digest, Sha256};

pub(crate) const SUMMARY_URL: &str =
    "https://maxinfosite-api-live.dragonflyathletics.com/schools/ZCUM49/summary";
pub(crate) const CAPTURE_DAY: &str = "2026-09-27";
pub(crate) const SUMMARY: &[u8] = include_bytes!(
    "../../../../census-crawl/tests/fixtures/coach_directories/nc_staff_summary_zcum49.json"
);
const DIRECTORY: &[u8] = include_bytes!(
    "../../../../census-crawl/tests/fixtures/coach_directories/nchsaa_directory_p1.json"
);

pub(crate) fn captured_school() -> CanonicalSchool {
    let summary: serde_json::Value = serde_json::from_slice(SUMMARY).unwrap();
    let directory: serde_json::Value = serde_json::from_slice(DIRECTORY).unwrap();
    let listing = &directory["results"][0];
    assert_eq!(listing["shortCode"], summary["shortCode"]);
    assert_eq!(listing["orgId"], summary["id"]);
    assert_eq!(summary["shortCode"], "ZCUM49");
    assert_eq!(listing["address"], summary["address"]["address1"]);
    let state = UsJurisdiction::parse(summary["address"]["state"].as_str().unwrap()).unwrap();
    let name = summary["name"].as_str().unwrap();
    let (mut school, _) =
        CanonicalSchool::new(state, name, census_domain::model::normalize_name(name));
    let owner = SourceIdentity::new(
        SourceNamespace::association_school("nchsaa"),
        summary["shortCode"].as_str().unwrap(),
    );
    school.source_identities.push(owner.clone());
    let source = &summary["address"];
    let address = PostalAddress::of(
        Some(StreetLine::parse(source["address1"].as_str().unwrap()).unwrap()),
        None,
        Some(CityName::parse(source["city"].as_str().unwrap()).unwrap()),
        Some(state),
        Some(ZipCode::parse(source["zip"].as_str().unwrap()).unwrap()),
    )
    .unwrap();
    school
        .add_postal_address(
            SchoolPostalAddress::new(
                address,
                owner,
                SourceLabel::AthleticAssociation { state },
                Evidence::parsed(
                    SourceRef::new("nchsaa", Some(SUMMARY_URL.into())),
                    CAPTURE_DAY,
                ),
                format!("{:x}", Sha256::digest(SUMMARY)),
            )
            .unwrap(),
        )
        .unwrap();
    school
}

#[test]
fn captured_postal_claim_preserves_raw_capture_provenance_and_published_components() {
    let school = captured_school();
    let fields = postal_fields([&school]).unwrap();
    assert_eq!(
        fields,
        [
            school.id.to_string(),
            "1 Rocket Drive".into(),
            "".into(),
            "Asheville".into(),
            "NC".into(),
            "28803".into(),
            "association_school:nchsaa".into(),
            "ZCUM49".into(),
            "athletic-association:NC".into(),
            SUMMARY_URL.into(),
            CAPTURE_DAY.into(),
            format!("{:x}", Sha256::digest(SUMMARY)),
        ]
    );
}

#[test]
fn conflicting_postal_claims_keep_their_corresponding_owner_capture_and_missing_components() {
    let mut school = captured_school();
    let owner = SourceIdentity::new(
        SourceNamespace::association_school("contradiction-fixture"),
        "alternate",
    );
    school.source_identities.push(owner.clone());
    let address = PostalAddress::of(
        Some(StreetLine::parse("2 Review Road").unwrap()),
        Some(StreetLine::parse("Building B").unwrap()),
        None,
        Some(UsJurisdiction::NorthCarolina),
        Some(ZipCode::parse("07030-0012").unwrap()),
    )
    .unwrap();
    school
        .add_postal_address(
            SchoolPostalAddress::new(
                address,
                owner,
                SourceLabel::AthleticAssociation {
                    state: UsJurisdiction::NorthCarolina,
                },
                Evidence::parsed(
                    SourceRef::new(
                        "contradiction-fixture",
                        Some("https://fixture.test/alternate".into()),
                    ),
                    "2026-09-28",
                ),
                "b".repeat(64),
            )
            .unwrap(),
        )
        .unwrap();
    let fields = postal_fields([&school]).unwrap();
    assert_eq!(fields[1], "1 Rocket Drive\n2 Review Road");
    assert_eq!(fields[2], "\nBuilding B");
    assert_eq!(fields[3], "Asheville\n");
    assert_eq!(fields[5], "28803\n07030-0012");
    assert_eq!(fields[7], "ZCUM49\nalternate");
    assert_eq!(
        fields[9],
        format!("{SUMMARY_URL}\nhttps://fixture.test/alternate")
    );
    assert_eq!(fields[10], "2026-09-27\n2026-09-28");
    assert_eq!(
        fields[11],
        format!("{:x}\n{}", Sha256::digest(SUMMARY), "b".repeat(64))
    );
}

#[test]
fn foreign_postal_owner_cannot_be_published_as_a_school_fact() {
    let mut school = captured_school();
    school.source_identities.clear();
    let error = postal_fields([&school]).unwrap_err().to_string();
    assert!(error.contains("requires review"), "{error}");
    assert!(error.contains("identity is not attached"), "{error}");
}

#[test]
fn school_city_and_state_do_not_supply_an_uncaptured_postal_address() {
    let mut school = captured_school();
    school.postal_addresses.clear();
    school.city = Some("Asheville".into());
    assert_eq!(
        postal_fields([&school]).unwrap(),
        std::array::from_fn(|_| String::new())
    );
}

fn budget_school(count: usize, owner_length: usize) -> CanonicalSchool {
    let mut school = captured_school();
    school.postal_addresses.clear();
    for index in 0..count {
        let id = format!("{index:03}{}", "x".repeat(owner_length - 3));
        let owner = SourceIdentity::new(SourceNamespace::association_school("postal-budget"), id);
        school.source_identities.push(owner.clone());
        let claim = SchoolPostalAddress::new(
            PostalAddress::line(StreetLine::parse("1 Budget Street").unwrap()),
            owner,
            SourceLabel::AthleticAssociation {
                state: UsJurisdiction::NorthCarolina,
            },
            Evidence::parsed(
                SourceRef::new("postal-budget", Some("https://fixture.test/budget".into())),
                CAPTURE_DAY,
            ),
            "a".repeat(64),
        )
        .unwrap();
        school.add_postal_address(claim).unwrap();
    }
    school
}

#[test]
fn maximum_postal_metadata_is_retained_and_an_additional_byte_is_refused() {
    let school = budget_school(128, 255);
    let fields = postal_fields([&school]).unwrap();
    let owners = school
        .postal_addresses
        .iter()
        .map(|claim| claim.owner().id.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(fields[7], owners);
    assert_eq!(fields[7].len(), 32_767);
    assert_eq!(fields[1], vec!["1 Budget Street"; 128].join("\n"));
    let mut overflow = school.clone();
    let owner = SourceIdentity::new(
        SourceNamespace::association_school("postal-budget"),
        "x".repeat(256),
    );
    overflow.source_identities.push(owner.clone());
    overflow.postal_addresses[0] = SchoolPostalAddress::new(
        PostalAddress::line(StreetLine::parse("1 Budget Street").unwrap()),
        owner,
        SourceLabel::AthleticAssociation {
            state: UsJurisdiction::NorthCarolina,
        },
        Evidence::parsed(
            SourceRef::new("postal-budget", Some("https://fixture.test/budget".into())),
            CAPTURE_DAY,
        ),
        "a".repeat(64),
    )
    .unwrap();
    assert!(matches!(
        postal_fields([&overflow]),
        Err(ReportError::Invariant { .. })
    ));
}

#[test]
fn a_postal_claim_over_the_publication_count_budget_is_refused_instead_of_dropped() {
    let maximum = budget_school(128, 3);
    let fields = postal_fields([&maximum]).unwrap();
    assert_eq!(
        fields[7],
        (0..128)
            .map(|index| format!("{index:03}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let overflow = budget_school(129, 3);
    assert!(matches!(
        postal_fields([&overflow]),
        Err(ReportError::Invariant { .. })
    ));
}
