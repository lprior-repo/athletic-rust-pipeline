use super::TestResult;
use crate::school_directory::{
    AssociationLabel, CityName, Coordinates, DirectoryKey, Enrollment, Grade, GradeSpan,
    IdentifiedKey, MatchForm, NcesSchoolId, Phone, PostalAddress, PssId, SchoolDirectoryEntry,
    SchoolKind, SchoolName, SourceLabel, StateRecordId, StreetLine, WeakKey, Website, ZipCode,
};
use crate::UsJurisdiction;

fn name(raw: &str) -> Result<SchoolName, Box<dyn std::error::Error>> {
    Ok(SchoolName::parse(raw)?)
}

fn city(raw: &str) -> Result<CityName, Box<dyn std::error::Error>> {
    Ok(CityName::parse(raw)?)
}

fn street(raw: &str) -> Result<StreetLine, Box<dyn std::error::Error>> {
    Ok(StreetLine::parse(raw)?)
}

fn zip(raw: &str) -> Result<ZipCode, Box<dyn std::error::Error>> {
    Ok(ZipCode::parse(raw)?)
}

fn address(
    line: &str,
    city_name: &str,
    state: UsJurisdiction,
    zip_code: &str,
) -> Result<PostalAddress, Box<dyn std::error::Error>> {
    Ok(PostalAddress::of(
        Some(street(line)?),
        None,
        Some(city(city_name)?),
        Some(state),
        Some(zip(zip_code)?),
    )
    .ok_or("address has parts")?)
}

fn street_of(entry: &SchoolDirectoryEntry) -> Result<&str, Box<dyn std::error::Error>> {
    Ok(entry
        .address()
        .and_then(PostalAddress::line1)
        .map(StreetLine::as_str)
        .ok_or("address line")?)
}

#[test]
fn published_identifiers_are_fixed_width_digits() -> TestResult {
    check!(eq; NcesSchoolId::parse(" 010001000001 ")?.as_str(),
    "010001000001");
    check!(NcesSchoolId::parse("01000100001").is_err());
    check!(NcesSchoolId::parse("01000100000A").is_err());

    check!(PssId::parse("01000100").is_ok());
    check!(eq; PssId::parse(" a2380006 ")?.as_str(), "A2380006");
    check!(PssId::parse("A238000").is_err());
    check!(PssId::parse("A238000-").is_err());

    check!(StateRecordId::parse("AL-0421").is_ok());
    check!(StateRecordId::parse("AL 0421").is_err());
    check!(StateRecordId::parse("").is_err());
    Ok(())
}

#[test]
fn match_form_folds_case_punctuation_and_spacing() -> Result<(), Box<dyn std::error::Error>> {
    check!(eq; MatchForm::of("St. Mary's High School").as_str(),
"st marys high school");
    check!(eq; MatchForm::of("  OAK   GROVE  ").as_str(), "oak grove");
    check!(eq; MatchForm::of("9th Grade Academy").as_str(),
"9th grade academy");
    check!(eq; MatchForm::of("...").as_str(), "");
    Ok(())
}

#[test]
fn weak_keys_need_a_name_and_carry_the_locality() -> TestResult {
    let key = WeakKey::of(
        &name("Albertville High School")?,
        Some(&city("Albertville")?),
        Some(UsJurisdiction::Alabama),
    )?;
    check!(eq; key.label(), "albertville high school|albertville|AL");
    check!(eq; key.name().as_str(), "albertville high school");
    check!(eq; key.state(), Some(UsJurisdiction::Alabama));

    check!(WeakKey::of(&name("...")?, None, None).is_err());

    let unlocated = WeakKey::of(&name("Albertville High School")?, None, None)?;
    check!(eq; unlocated.label(), "albertville high school||");
    check!(ne; unlocated, key);
    Ok(())
}

#[test]
fn directory_keys_rank_identity_strength() -> TestResult {
    let nces = DirectoryKey::from(IdentifiedKey::Nces(NcesSchoolId::parse("010001000001")?));
    let pss = DirectoryKey::from(IdentifiedKey::Pss(PssId::parse("01000100")?));
    let state = DirectoryKey::from(IdentifiedKey::StateRecord {
        state: UsJurisdiction::Alabama,
        id: StateRecordId::parse("AL-0421")?,
    });
    let weak = DirectoryKey::Weak(WeakKey::of(&name("Albertville High School")?, None, None)?);

    check!(eq; nces.rank(), 0);
    check!(eq; pss.rank(), 1);
    check!(eq; state.rank(), 2);
    check!(eq; weak.rank(), 3);
    check!(eq; nces.label(), "nces:010001000001");
    check!(eq; pss.label(), "pss:01000100");
    check!(eq; state.label(), "state:AL:AL-0421");
    check!(weak.label().starts_with("weak:albertville"));
    Ok(())
}

#[test]
fn absorb_keeps_the_stronger_source_value() -> TestResult {
    let census = SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse("010001000001")?),
        SourceLabel::Ccd,
        Some(name("ALBERTVILLE HIGH SCHOOL")?),
    )
    .with_address(Some(address(
        "600 E Alabama Ave",
        "ALBERTVILLE",
        UsJurisdiction::Alabama,
        "35950",
    )?));

    let association = SchoolDirectoryEntry::weak(
        name("Albertville High School")?,
        Some(city("Albertville")?),
        Some(UsJurisdiction::Alabama),
        SourceLabel::AthleticAssociation {
            state: UsJurisdiction::Alabama,
        },
    )?
    .with_address(Some(address(
        "600 East Alabama Avenue",
        "Albertville",
        UsJurisdiction::Alabama,
        "35950-2336",
    )?));

    let mut census_first = census.clone();
    census_first.absorb(&association);
    check!(eq; census_first.name().ok_or("name")?.as_str(),
    "ALBERTVILLE HIGH SCHOOL");
    check!(eq; street_of(&census_first)?, "600 E Alabama Ave");
    check!(eq; census_first.key().rank(), 0);
    check!(eq; census_first.sources().len(), 2);

    let mut association_first = association;
    association_first.absorb(&census);
    check!(eq; association_first.name().ok_or("name")?.as_str(),
    "ALBERTVILLE HIGH SCHOOL");
    check!(eq; street_of(&association_first)?, "600 E Alabama Ave");
    check!(eq; association_first.key(), census_first.key());
    Ok(())
}

#[test]
fn absorb_fills_gaps_and_settles_equal_priority_deterministically() -> TestResult {
    let key = IdentifiedKey::Nces(NcesSchoolId::parse("010001000001")?);
    let mut entry = SchoolDirectoryEntry::identified(key.clone(), SourceLabel::Ccd, None)
        .with_enrollment(Enrollment::parse("512")?);
    let counterpart = SchoolDirectoryEntry::identified(key, SourceLabel::Ccd, Some(name("A H S")?))
        .with_grades(Some(GradeSpan::new(
            Grade::parse("9")?.ok_or("present")?,
            Grade::parse("12")?.ok_or("present")?,
        )?))
        .with_phone(Phone::parse("(256) 878-1234")?);

    entry.absorb(&counterpart);
    check!(eq; entry.name().ok_or("name")?.as_str(), "A H S");
    check!(eq; entry.enrollment().ok_or("enrollment")?.get(), 512);
    check!(eq; entry.grades().ok_or("grades")?.label(), "9-12");
    check!(eq; entry.phone().ok_or("phone")?.as_str(), "(256) 878-1234");
    check!(eq; entry.sources().len(), 1);

    let shouted = SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse("010001000001")?),
        SourceLabel::Ccd,
        Some(name("ALBERTVILLE HIGH SCHOOL")?),
    );
    let mut settled = shouted.clone();
    settled.absorb(&SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse("010001000001")?),
        SourceLabel::Ccd,
        Some(name("Albertville High School")?),
    ));
    let mut reversed = SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse("010001000001")?),
        SourceLabel::Ccd,
        Some(name("Albertville High School")?),
    );
    reversed.absorb(&shouted);
    check!(eq; settled.name(), reversed.name());
    Ok(())
}

#[test]
fn grades_enrollment_and_contacts_parse_the_published_forms() -> TestResult {
    check!(eq; Grade::parse("PK")?, Some(Grade::PreK));
    check!(eq; Grade::parse("k")?, Some(Grade::Kindergarten));
    check!(eq; Grade::parse(" 12 ")?.map(Grade::label),
    Some("12".to_string()));
    check!(eq; Grade::parse("N")?, None);
    check!(eq; Grade::parse("")?, None);
    check!(Grade::parse("13").is_err());
    check!(Grade::parse("college").is_err());

    check!(Enrollment::parse(" 1,200 ").is_err());
    check!(eq; Enrollment::parse("1200")?.map(Enrollment::get), Some(1200));
    check!(eq; Enrollment::parse("   ")?, None);

    let ninth = Grade::parse("9")?.ok_or("present")?;
    let twelfth = Grade::parse("12")?.ok_or("present")?;
    let span = GradeSpan::new(ninth, twelfth)?;
    check!(eq; span.label(), "9-12");
    check!(GradeSpan::new(twelfth, ninth).is_err());
    check!(GradeSpan::new(Grade::Ungraded, ninth).is_err());

    check!(eq; Phone::parse("(256) 878-1234")?.ok_or("present")?.as_str(),
    "(256) 878-1234");
    check!(eq; Phone::parse("   ")?, None);
    check!(Phone::parse("12345").is_err());
    check!(Phone::parse("call the office").is_err());

    check!(eq; Website::parse("https://www.albertvillek12.org")?
        .ok_or("present")?
        .as_str(),
    "https://www.albertvillek12.org");
    check!(eq; Website::parse("")?, None);
    check!(Website::parse("www.example.org").is_err());
    check!(Website::parse("https://example.org/has space").is_err());
    Ok(())
}

#[test]
fn school_kinds_render_their_affiliation() -> TestResult {
    check!(eq; SchoolKind::public().label(), "Public");
    check!(eq; SchoolKind::charter().label(), "Charter");
    check!(eq; SchoolKind::private().label(), "Private");
    let affiliated = SchoolKind::affiliated(AssociationLabel::parse("AHSAA")?);
    check!(eq; affiliated.label(), "Private (AHSAA)");
    Ok(())
}

#[test]
fn geocoded_coordinates_are_stamped_weakest_and_never_displace_a_published_source() -> TestResult {
    let geocoded = Coordinates::parse("33.4699825", "-86.9245195")?;
    let published = Coordinates::parse("34.0000001", "-86.0000002")?;
    let key = || NcesSchoolId::parse("010001000001").map(IdentifiedKey::Nces);

    let mut entry = SchoolDirectoryEntry::identified(
        key()?,
        SourceLabel::AthleticAssociation {
            state: UsJurisdiction::Alabama,
        },
        Some(name("Albertville High School")?),
    );
    entry.set_coordinates_from(geocoded, SourceLabel::Geocoder);
    check!(eq; entry.coordinates(), Some(geocoded));
    check!(entry.sources().contains(&SourceLabel::Geocoder));
    check!(eq; SourceLabel::Geocoder.rank(), 5);
    check!(eq; SourceLabel::Geocoder.label(), "geocoder");
    check!(
        SourceLabel::Geocoder.rank()
            > SourceLabel::AthleticAssociation {
                state: UsJurisdiction::Alabama
            }
            .rank()
    );

    let artifact = SchoolDirectoryEntry::identified(
        key()?,
        SourceLabel::Ccd,
        Some(name("Albertville High School")?),
    )
    .with_coordinates(Some(published));
    entry.absorb(&artifact);
    check!(eq; entry.coordinates(), Some(published));

    let json = serde_json::to_string(&SourceLabel::Geocoder)?;
    check!(eq; json, "\"Geocoder\"");
    check!(eq; serde_json::from_str::<SourceLabel>(&json)?,
    SourceLabel::Geocoder);
    Ok(())
}
