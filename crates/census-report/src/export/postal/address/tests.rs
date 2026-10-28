use super::*;
use crate::export::postal::postal_fields;
use crate::export::postal::tests::{captured_school, CAPTURE_DAY, SUMMARY_URL};
use census_domain::model::{Evidence, SchoolPostalAddress, SourceIdentity, SourceRef};
use census_domain::school_directory::{AddressKind, CityName, PostalAddress, StreetLine, ZipCode};
use census_domain::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const CAPTURED_ADDRESS: &str = "1 Rocket Drive, Asheville, NC 28803";

fn retain(school: &mut CanonicalSchool, address: PostalAddress, observed_on: &str) -> TestResult {
    let original = school.postal_addresses.first().ok_or("fixture claim")?;
    let claim = SchoolPostalAddress::new(
        address,
        original.owner().clone(),
        original.source_label().clone(),
        Evidence::parsed(
            SourceRef::new("nchsaa", Some(SUMMARY_URL.into())),
            observed_on,
        ),
        "b".repeat(64),
    )?;
    school.add_postal_address(claim)?;
    Ok(())
}

#[test]
fn partial_claim_does_not_hide_a_retained_street_or_borrow_its_components() -> TestResult {
    let mut school = captured_school()?;
    let partial = PostalAddress::of(
        None,
        None,
        Some(CityName::parse("Asheville")?),
        Some(UsJurisdiction::NorthCarolina),
        None,
    )
    .ok_or("partial tuple")?;
    retain(&mut school, partial, "2026-09-26")?;
    check!(eq; address_line([&school])?, format!("Asheville, NC\n{CAPTURED_ADDRESS}"));
    let fields = postal_fields([&school])?;
    check!(eq; fields.get(1).map(String::as_str), Some("\n1 Rocket Drive"));
    check!(eq; fields.get(5).map(String::as_str), Some("\n28803"));
    check!(eq; fields.get(10).map(String::as_str), Some("2026-09-26\n2026-09-27"));
    Ok(())
}

#[test]
fn conflicting_retained_tuples_are_not_replaced_by_one_lexical_or_newer_winner() -> TestResult {
    let mut school = captured_school()?;
    retain(
        &mut school,
        PostalAddress::line(StreetLine::parse("2 Review Road")?),
        "2026-10-01",
    )?;
    let expected = format!("{CAPTURED_ADDRESS}\n2 Review Road");
    check!(eq; address_line([&school])?, expected);
    school.postal_addresses.reverse();
    check!(eq; address_line([&school])?, expected);
    Ok(())
}

#[test]
fn repeated_tuple_is_compact_once_without_dropping_capture_provenance_or_freshness() -> TestResult {
    let mut school = captured_school()?;
    let address = school
        .postal_addresses
        .first()
        .ok_or("fixture claim")?
        .address()
        .clone();
    retain(&mut school, address, "2026-10-01T10:00:00Z")?;
    let before = serde_json::to_vec(&school)?;
    let fields = postal_fields([&school])?;
    check!(eq; fields.get(1).map(String::as_str), Some("1 Rocket Drive\n1 Rocket Drive"));
    check!(eq; fields.get(10).map(String::as_str), Some("2026-09-27\n2026-10-01T10:00:00Z"));
    let restored: CanonicalSchool = serde_json::from_slice(&before)?;
    for _ in 0..3 {
        check!(eq; address_line([&restored])?, CAPTURED_ADDRESS);
        check!(eq; postal_fields([&restored])?, fields);
        check!(eq; serde_json::to_vec(&restored)?, before);
    }
    Ok(())
}

#[test]
fn no_claim_remains_unknown_even_when_an_unrelated_same_name_school_has_an_address() -> TestResult {
    let school = captured_school()?;
    let (absent, _) = CanonicalSchool::new(
        UsJurisdiction::NorthCarolina,
        &school.name,
        &school.normalized_name,
        Some("Asheville"),
    );
    check!(eq; absent.name, school.name);
    check!(ne; absent.id, school.id);
    check!(eq; address_line([&absent])?, "");
    check!(eq; address_line(std::iter::empty::<&CanonicalSchool>())?, "");
    check!(eq; address_line([&school])?, CAPTURED_ADDRESS);
    Ok(())
}

#[test]
fn an_unowned_later_claim_refuses_the_whole_compact_cell() -> TestResult {
    let mut school = captured_school()?;
    let original = school.postal_addresses.first().ok_or("fixture claim")?;
    let foreign = SchoolPostalAddress::new(
        PostalAddress::line(StreetLine::parse("2 Review Road")?),
        SourceIdentity::new(original.owner().namespace.clone(), "unattached"),
        original.source_label().clone(),
        Evidence::parsed(
            SourceRef::new("nchsaa", Some(SUMMARY_URL.into())),
            CAPTURE_DAY,
        ),
        "b".repeat(64),
    )?;
    school.postal_addresses.push(foreign);
    let error = match address_line([&school]) {
        Err(error) => error.to_string(),
        Ok(_) => return Err("unowned compact address published".into()),
    };
    check!(error.contains("requires review"), "{error}");
    Ok(())
}

#[test]
fn identical_text_with_distinct_address_kind_is_not_a_duplicate_tuple() -> TestResult {
    let mut school = captured_school()?;
    let address = school
        .postal_addresses
        .first()
        .ok_or("fixture claim")?
        .address()
        .clone();
    retain(
        &mut school,
        address.with_kind(AddressKind::Physical),
        CAPTURE_DAY,
    )?;
    check!(eq; address_line([&school])?, format!("{CAPTURED_ADDRESS}\n{CAPTURED_ADDRESS}"));
    let fields = postal_fields([&school])?;
    check!(eq; fields.get(12).map(String::as_str), Some("physical\nunknown"));
    Ok(())
}

#[test]
fn claim_budget_is_enforced_before_compact_deduplication() -> TestResult {
    let mut school = captured_school()?;
    let claim = school
        .postal_addresses
        .first()
        .ok_or("fixture claim")?
        .clone();
    school.postal_addresses = vec![claim.clone(); 128];
    check!(eq; address_line([&school])?, CAPTURED_ADDRESS);
    school.postal_addresses.push(claim);
    let error = match address_line([&school]) {
        Err(error) => error.to_string(),
        Ok(_) => return Err("over-budget claims published".into()),
    };
    check!(error.contains("128"), "{error}");
    Ok(())
}

#[test]
fn utf16_cell_limit_counts_surrogate_pairs_and_newlines_not_utf8_bytes() -> TestResult {
    let mut cell = String::new();
    let mut units = 0;
    append_line(&mut cell, &mut units, &"😀".repeat(16_382))?;
    append_line(&mut cell, &mut units, "😀")?;
    check!(eq; units, CELL_UTF16_LIMIT);
    check!(eq; cell.encode_utf16().count(), CELL_UTF16_LIMIT);
    check!(cell.len() > CELL_UTF16_LIMIT);
    let before = cell.clone();
    check!(append_line(&mut cell, &mut units, "x").is_err());
    check!(eq; cell, before);
    check!(eq; units, CELL_UTF16_LIMIT);
    Ok(())
}

#[test]
fn utf16_limit_refuses_a_single_extra_code_unit_without_partial_write() -> TestResult {
    let mut cell = String::new();
    let mut units = 0;
    check!(append_line(&mut cell, &mut units, &"😀".repeat(16_384)).is_err());
    check!(cell.is_empty());
    check!(eq; units, 0);
    append_line(&mut cell, &mut units, &"x".repeat(CELL_UTF16_LIMIT))?;
    check!(eq; cell.len(), CELL_UTF16_LIMIT);
    Ok(())
}

#[test]
fn retained_address_tuple_does_not_combine_mailing_and_physical_components() -> TestResult {
    let mut school = captured_school()?;
    let mailing = PostalAddress::of(
        Some(StreetLine::parse("PO Box 12")?),
        Some(StreetLine::parse("Mail Suite")?),
        Some(CityName::parse("New York")?),
        Some(UsJurisdiction::NewYork),
        Some(ZipCode::parse("10005-6666")?),
    )
    .ok_or("mailing tuple")?
    .with_kind(AddressKind::Mailing);
    retain(&mut school, mailing, CAPTURE_DAY)?;
    check!(eq; address_line([&school])?, format!("PO Box 12, Mail Suite, New York, NY 10005-6666\n{CAPTURED_ADDRESS}"));
    Ok(())
}

#[test]
fn aggregate_tuple_output_refuses_over_limit_instead_of_truncating_claims() -> TestResult {
    let mut school = captured_school()?;
    for index in 0..128 {
        let address = PostalAddress::of(
            Some(StreetLine::parse(&format!(
                "{index:03} {}",
                "x".repeat(116)
            ))?),
            Some(StreetLine::parse(&"y".repeat(120))?),
            Some(CityName::parse(&"z".repeat(64))?),
            Some(UsJurisdiction::NorthCarolina),
            None,
        )
        .ok_or("large valid tuple")?;
        retain(&mut school, address, CAPTURE_DAY)?;
    }
    school
        .postal_addresses
        .retain(|claim| claim.address().line2().is_some());
    check!(eq; school.postal_addresses.len(), 128);
    let before = serde_json::to_vec(&school)?;
    let error = match address_line([&school]) {
        Err(error) => error.to_string(),
        Ok(_) => return Err("over-budget compact cell published".into()),
    };
    check!(error.contains("UTF-16 cell budget"), "{error}");
    check!(eq; serde_json::to_vec(&school)?, before);
    Ok(())
}
