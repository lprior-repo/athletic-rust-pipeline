use super::*;
use census_domain::model::{Evidence, SourceIdentity, SourceNamespace, SourceRef};
use census_domain::school_directory::{CityName, PostalAddress, SourceLabel, StreetLine, ZipCode};
use census_domain::UsJurisdiction;
use sha2::{Digest, Sha256};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(crate) const SUMMARY_URL: &str =
    "https://maxinfosite-api-live.dragonflyathletics.com/schools/ZCUM49/summary";
pub(crate) const CAPTURE_DAY: &str = "2026-09-27";
pub(crate) const SUMMARY: &[u8] = include_bytes!(
    "../../../../census-crawl/tests/fixtures/coach_directories/nc_staff_summary_zcum49.json"
);
const DIRECTORY: &[u8] = include_bytes!(
    "../../../../census-crawl/tests/fixtures/coach_directories/nchsaa_directory_p1.json"
);

pub(crate) fn captured_school() -> TestResult<CanonicalSchool> {
    let summary: serde_json::Value = serde_json::from_slice(SUMMARY)?;
    let directory: serde_json::Value = serde_json::from_slice(DIRECTORY)?;
    let listing = &directory["results"][0];
    check!(eq; listing["shortCode"], summary["shortCode"]);
    check!(eq; listing["orgId"], summary["id"]);
    check!(eq; summary["shortCode"], "ZCUM49");
    check!(eq; listing["address"], summary["address"]["address1"]);
    let state = UsJurisdiction::parse(
        summary["address"]["state"]
            .as_str()
            .ok_or("capture state is not text")?,
    )
    .ok_or("capture state is invalid")?;
    let name = summary["name"]
        .as_str()
        .ok_or("capture school name is not text")?;
    let (mut school, _) = CanonicalSchool::new(
        state,
        name,
        census_domain::model::normalize_name(name),
        None,
    );
    let owner = SourceIdentity::new(
        SourceNamespace::association_school("nchsaa"),
        summary["shortCode"]
            .as_str()
            .ok_or("capture short code is not text")?,
    );
    school.source_identities.push(owner.clone());
    let source = &summary["address"];
    let address = PostalAddress::of(
        Some(StreetLine::parse(
            source["address1"]
                .as_str()
                .ok_or("capture street is not text")?,
        )?),
        None,
        Some(CityName::parse(
            source["city"].as_str().ok_or("capture city is not text")?,
        )?),
        Some(state),
        Some(ZipCode::parse(
            source["zip"].as_str().ok_or("capture ZIP is not text")?,
        )?),
    )
    .ok_or("captured postal fixture has no address components")?;
    school.add_postal_address(SchoolPostalAddress::new(
        address,
        owner,
        SourceLabel::AthleticAssociation { state },
        Evidence::parsed(
            SourceRef::new("nchsaa", Some(SUMMARY_URL.into())),
            CAPTURE_DAY,
        ),
        format!("{:x}", Sha256::digest(SUMMARY)),
    )?)?;
    Ok(school)
}

#[test]
fn captured_postal_claim_preserves_raw_capture_provenance_and_published_components() -> TestResult {
    let school = captured_school()?;
    let fields = postal_fields([&school])?;
    check!(eq; fields,
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
    ]);
    Ok(())
}

#[test]
fn conflicting_postal_claims_keep_their_corresponding_owner_capture_and_missing_components(
) -> TestResult {
    let mut school = captured_school()?;
    let owner = SourceIdentity::new(
        SourceNamespace::association_school("contradiction-fixture"),
        "alternate",
    );
    school.source_identities.push(owner.clone());
    let address = PostalAddress::of(
        Some(StreetLine::parse("2 Review Road")?),
        Some(StreetLine::parse("Building B")?),
        None,
        Some(UsJurisdiction::NorthCarolina),
        Some(ZipCode::parse("07030-0012")?),
    )
    .ok_or("disputed postal fixture has no address components")?;
    school.add_postal_address(SchoolPostalAddress::new(
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
    )?)?;
    school.postal_addresses.reverse();
    let fields = postal_fields([&school])?;
    check!(eq; fields[1], "1 Rocket Drive\n2 Review Road");
    check!(eq; fields[2], "\nBuilding B");
    check!(eq; fields[3], "Asheville\n");
    check!(eq; fields[5], "28803\n07030-0012");
    check!(eq; fields[7], "ZCUM49\nalternate");
    check!(eq; fields[9],
    format!("{SUMMARY_URL}\nhttps://fixture.test/alternate"));
    check!(eq; fields[10], "2026-09-27\n2026-09-28");
    check!(eq; fields[11],
    format!("{:x}\n{}", Sha256::digest(SUMMARY), "b".repeat(64)));
    Ok(())
}

#[test]
fn foreign_postal_owner_cannot_be_published_as_a_school_fact() -> TestResult {
    let mut school = captured_school()?;
    school.source_identities.clear();
    let error = match postal_fields([&school]) {
        Err(error) => error.to_string(),
        Ok(_) => return Err("foreign postal owner was published".into()),
    };
    check!(error.contains("requires review"), "{error}");
    check!(error.contains("identity is not attached"), "{error}");
    Ok(())
}

#[test]
fn school_city_and_state_do_not_supply_an_uncaptured_postal_address() -> TestResult {
    let mut school = captured_school()?;
    school.postal_addresses.clear();
    school.city = Some("Asheville".into());
    check!(eq; postal_fields([&school])?,
    std::array::from_fn(|_| String::new()));
    Ok(())
}

fn budget_school(count: usize, owner_length: usize) -> TestResult<CanonicalSchool> {
    let mut school = captured_school()?;
    school.postal_addresses.clear();
    for index in 0..count {
        let id = format!("{index:03}{}", "x".repeat(owner_length - 3));
        let owner = SourceIdentity::new(SourceNamespace::association_school("postal-budget"), id);
        school.source_identities.push(owner.clone());
        let claim = SchoolPostalAddress::new(
            PostalAddress::line(StreetLine::parse("1 Budget Street")?),
            owner,
            SourceLabel::AthleticAssociation {
                state: UsJurisdiction::NorthCarolina,
            },
            Evidence::parsed(
                SourceRef::new("postal-budget", Some("https://fixture.test/budget".into())),
                CAPTURE_DAY,
            ),
            "a".repeat(64),
        )?;
        school.add_postal_address(claim)?;
    }
    Ok(school)
}

#[test]
fn maximum_postal_metadata_is_retained_and_an_additional_byte_is_refused() -> TestResult {
    let school = budget_school(128, 255)?;
    let fields = postal_fields([&school])?;
    let owners = school
        .postal_addresses
        .iter()
        .map(|claim| claim.owner().id.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    check!(eq; fields[7], owners);
    check!(eq; fields[7].len(), 32_767);
    check!(eq; fields[1], vec!["1 Budget Street"; 128].join("\n"));
    let mut overflow = school.clone();
    let owner = SourceIdentity::new(
        SourceNamespace::association_school("postal-budget"),
        "x".repeat(256),
    );
    overflow.source_identities.push(owner.clone());
    overflow.postal_addresses[0] = SchoolPostalAddress::new(
        PostalAddress::line(StreetLine::parse("1 Budget Street")?),
        owner,
        SourceLabel::AthleticAssociation {
            state: UsJurisdiction::NorthCarolina,
        },
        Evidence::parsed(
            SourceRef::new("postal-budget", Some("https://fixture.test/budget".into())),
            CAPTURE_DAY,
        ),
        "a".repeat(64),
    )?;
    check!(matches!(
        postal_fields([&overflow]),
        Err(ReportError::Invariant { .. })
    ));
    Ok(())
}

#[test]
fn a_postal_claim_over_the_publication_count_budget_is_refused_instead_of_dropped() -> TestResult {
    let maximum = budget_school(128, 3)?;
    let fields = postal_fields([&maximum])?;
    check!(eq; fields[7],
    (0..128)
        .map(|index| format!("{index:03}"))
        .collect::<Vec<_>>()
        .join("\n"));
    let overflow = budget_school(129, 3)?;
    check!(matches!(
        postal_fields([&overflow]),
        Err(ReportError::Invariant { .. })
    ));
    Ok(())
}
