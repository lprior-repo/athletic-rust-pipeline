use crate::school_directory::{
    AssociationLabel, CityName, Coordinates, DirectoryKey, Enrollment, Grade, GradeSpan,
    IdentifiedKey, MatchForm, NcesSchoolId, Phone, PostalAddress, PssId, SchoolDirectoryEntry,
    SchoolKind, SchoolName, SourceLabel, StateRecordId, StreetLine, WeakKey, Website, ZipCode,
};
use crate::UsJurisdiction;

fn name(raw: &str) -> SchoolName {
    SchoolName::parse(raw).expect("school name parses")
}

fn city(raw: &str) -> CityName {
    CityName::parse(raw).expect("city parses")
}

fn street(raw: &str) -> StreetLine {
    StreetLine::parse(raw).expect("street parses")
}

fn zip(raw: &str) -> ZipCode {
    ZipCode::parse(raw).expect("zip parses")
}

fn address(line: &str, city_name: &str, state: UsJurisdiction, zip_code: &str) -> PostalAddress {
    PostalAddress::of(
        Some(street(line)),
        None,
        Some(city(city_name)),
        Some(state),
        Some(zip(zip_code)),
    )
    .expect("address has parts")
}

fn street_of(entry: &SchoolDirectoryEntry) -> &str {
    entry
        .address()
        .and_then(PostalAddress::line1)
        .map(StreetLine::as_str)
        .expect("address line")
}

#[test]
fn published_identifiers_are_fixed_width_digits() {
    assert_eq!(
        NcesSchoolId::parse(" 010001000001 ")
            .expect("twelve digits parse")
            .as_str(),
        "010001000001"
    );
    assert!(NcesSchoolId::parse("01000100001").is_err());
    assert!(NcesSchoolId::parse("01000100000A").is_err());

    assert!(PssId::parse("01000100").is_ok());
    assert_eq!(
        PssId::parse(" a2380006 ")
            .expect("published PPIN parses")
            .as_str(),
        "A2380006"
    );
    assert!(PssId::parse("A238000").is_err());
    assert!(PssId::parse("A238000-").is_err());

    assert!(StateRecordId::parse("AL-0421").is_ok());
    assert!(StateRecordId::parse("AL 0421").is_err());
    assert!(StateRecordId::parse("").is_err());
}

#[test]
fn match_form_folds_case_punctuation_and_spacing() {
    assert_eq!(
        MatchForm::of("St. Mary's High School").as_str(),
        "st marys high school"
    );
    assert_eq!(MatchForm::of("  OAK   GROVE  ").as_str(), "oak grove");
    assert_eq!(
        MatchForm::of("9th Grade Academy").as_str(),
        "9th grade academy"
    );
    assert_eq!(MatchForm::of("...").as_str(), "");
}

#[test]
fn weak_keys_need_a_name_and_carry_the_locality() {
    let key = WeakKey::of(
        &name("Albertville High School"),
        Some(&city("Albertville")),
        Some(UsJurisdiction::Alabama),
    )
    .expect("weak key builds");
    assert_eq!(key.label(), "albertville high school|albertville|AL");
    assert_eq!(key.name().as_str(), "albertville high school");
    assert_eq!(key.state(), Some(UsJurisdiction::Alabama));

    assert!(WeakKey::of(&name("..."), None, None).is_err());

    let unlocated =
        WeakKey::of(&name("Albertville High School"), None, None).expect("a name alone is enough");
    assert_eq!(unlocated.label(), "albertville high school||");
    assert_ne!(unlocated, key);
}

#[test]
fn directory_keys_rank_identity_strength() {
    let nces = DirectoryKey::from(IdentifiedKey::Nces(
        NcesSchoolId::parse("010001000001").expect("nces id"),
    ));
    let pss = DirectoryKey::from(IdentifiedKey::Pss(
        PssId::parse("01000100").expect("pss id"),
    ));
    let state = DirectoryKey::from(IdentifiedKey::StateRecord {
        state: UsJurisdiction::Alabama,
        id: StateRecordId::parse("AL-0421").expect("state id"),
    });
    let weak = DirectoryKey::Weak(
        WeakKey::of(&name("Albertville High School"), None, None).expect("weak key"),
    );

    assert_eq!(nces.rank(), 0);
    assert_eq!(pss.rank(), 1);
    assert_eq!(state.rank(), 2);
    assert_eq!(weak.rank(), 3);
    assert_eq!(nces.label(), "nces:010001000001");
    assert_eq!(pss.label(), "pss:01000100");
    assert_eq!(state.label(), "state:AL:AL-0421");
    assert!(weak.label().starts_with("weak:albertville"));
}

#[test]
fn absorb_keeps_the_stronger_source_value() {
    let census = SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse("010001000001").expect("nces id")),
        SourceLabel::Ccd,
        Some(name("ALBERTVILLE HIGH SCHOOL")),
    )
    .with_address(Some(address(
        "600 E Alabama Ave",
        "ALBERTVILLE",
        UsJurisdiction::Alabama,
        "35950",
    )));

    let association = SchoolDirectoryEntry::weak(
        name("Albertville High School"),
        Some(city("Albertville")),
        Some(UsJurisdiction::Alabama),
        SourceLabel::AthleticAssociation {
            state: UsJurisdiction::Alabama,
        },
    )
    .expect("weak entry")
    .with_address(Some(address(
        "600 East Alabama Avenue",
        "Albertville",
        UsJurisdiction::Alabama,
        "35950-2336",
    )));

    let mut census_first = census.clone();
    census_first.absorb(&association);
    assert_eq!(
        census_first.name().expect("name").as_str(),
        "ALBERTVILLE HIGH SCHOOL"
    );
    assert_eq!(street_of(&census_first), "600 E Alabama Ave");
    assert_eq!(census_first.key().rank(), 0);
    assert_eq!(census_first.sources().len(), 2);

    let mut association_first = association;
    association_first.absorb(&census);
    assert_eq!(
        association_first.name().expect("name").as_str(),
        "ALBERTVILLE HIGH SCHOOL"
    );
    assert_eq!(street_of(&association_first), "600 E Alabama Ave");
    assert_eq!(association_first.key(), census_first.key());
}

#[test]
fn absorb_fills_gaps_and_settles_equal_priority_deterministically() {
    let key = IdentifiedKey::Nces(NcesSchoolId::parse("010001000001").expect("nces id"));
    let mut entry = SchoolDirectoryEntry::identified(key.clone(), SourceLabel::Ccd, None)
        .with_enrollment(Enrollment::parse("512").expect("enrollment"));
    let counterpart = SchoolDirectoryEntry::identified(key, SourceLabel::Ccd, Some(name("A H S")))
        .with_grades(Some(
            GradeSpan::new(
                Grade::parse("9").expect("grade").expect("present"),
                Grade::parse("12").expect("grade").expect("present"),
            )
            .expect("span"),
        ))
        .with_phone(Phone::parse("(256) 878-1234").expect("phone"));

    entry.absorb(&counterpart);
    assert_eq!(entry.name().expect("name").as_str(), "A H S");
    assert_eq!(entry.enrollment().expect("enrollment").get(), 512);
    assert_eq!(entry.grades().expect("grades").label(), "9-12");
    assert_eq!(entry.phone().expect("phone").as_str(), "(256) 878-1234");
    assert_eq!(entry.sources().len(), 1);

    let shouted = SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse("010001000001").expect("nces id")),
        SourceLabel::Ccd,
        Some(name("ALBERTVILLE HIGH SCHOOL")),
    );
    let mut settled = shouted.clone();
    settled.absorb(&SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse("010001000001").expect("nces id")),
        SourceLabel::Ccd,
        Some(name("Albertville High School")),
    ));
    let mut reversed = SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse("010001000001").expect("nces id")),
        SourceLabel::Ccd,
        Some(name("Albertville High School")),
    );
    reversed.absorb(&shouted);
    assert_eq!(settled.name(), reversed.name());
}

#[test]
fn grades_enrollment_and_contacts_parse_the_published_forms() {
    assert_eq!(Grade::parse("PK").expect("pk"), Some(Grade::PreK));
    assert_eq!(Grade::parse("k").expect("k"), Some(Grade::Kindergarten));
    assert_eq!(
        Grade::parse(" 12 ").expect("12").map(Grade::label),
        Some("12".to_string())
    );
    assert_eq!(Grade::parse("N").expect("absent"), None);
    assert_eq!(Grade::parse("").expect("blank"), None);
    assert!(Grade::parse("13").is_err());
    assert!(Grade::parse("college").is_err());

    assert!(Enrollment::parse(" 1,200 ").is_err());
    assert_eq!(
        Enrollment::parse("1200")
            .expect("enrollment")
            .map(Enrollment::get),
        Some(1200)
    );
    assert_eq!(Enrollment::parse("   ").expect("blank"), None);

    let ninth = Grade::parse("9").expect("grade").expect("present");
    let twelfth = Grade::parse("12").expect("grade").expect("present");
    let span = GradeSpan::new(ninth, twelfth).expect("span");
    assert_eq!(span.label(), "9-12");
    assert!(GradeSpan::new(twelfth, ninth).is_err());
    assert!(GradeSpan::new(Grade::Ungraded, ninth).is_err());

    assert_eq!(
        Phone::parse("(256) 878-1234")
            .expect("phone")
            .expect("present")
            .as_str(),
        "(256) 878-1234"
    );
    assert_eq!(Phone::parse("   ").expect("blank"), None);
    assert!(Phone::parse("12345").is_err());
    assert!(Phone::parse("call the office").is_err());

    assert_eq!(
        Website::parse("https://www.albertvillek12.org")
            .expect("website")
            .expect("present")
            .as_str(),
        "https://www.albertvillek12.org"
    );
    assert_eq!(Website::parse("").expect("blank"), None);
    assert!(Website::parse("www.example.org").is_err());
    assert!(Website::parse("https://example.org/has space").is_err());
}

#[test]
fn school_kinds_render_their_affiliation() {
    assert_eq!(SchoolKind::public().label(), "Public");
    assert_eq!(SchoolKind::charter().label(), "Charter");
    assert_eq!(SchoolKind::private().label(), "Private");
    let affiliated = SchoolKind::affiliated(AssociationLabel::parse("AHSAA").expect("label"));
    assert_eq!(affiliated.label(), "Private (AHSAA)");
}

#[test]
fn geocoded_coordinates_are_stamped_weakest_and_never_displace_a_published_source() {
    let geocoded = Coordinates::parse("33.4699825", "-86.9245195").expect("geocoded coordinates");
    let published = Coordinates::parse("34.0000001", "-86.0000002").expect("published coordinates");
    let key = || IdentifiedKey::Nces(NcesSchoolId::parse("010001000001").expect("nces id"));

    let mut entry = SchoolDirectoryEntry::identified(
        key(),
        SourceLabel::AthleticAssociation {
            state: UsJurisdiction::Alabama,
        },
        Some(name("Albertville High School")),
    );
    entry.set_coordinates_from(geocoded, SourceLabel::Geocoder);
    assert_eq!(entry.coordinates(), Some(geocoded));
    assert!(entry.sources().contains(&SourceLabel::Geocoder));
    assert_eq!(SourceLabel::Geocoder.rank(), 5);
    assert_eq!(SourceLabel::Geocoder.label(), "geocoder");
    assert!(
        SourceLabel::Geocoder.rank()
            > SourceLabel::AthleticAssociation {
                state: UsJurisdiction::Alabama
            }
            .rank()
    );

    let artifact = SchoolDirectoryEntry::identified(
        key(),
        SourceLabel::Ccd,
        Some(name("Albertville High School")),
    )
    .with_coordinates(Some(published));
    entry.absorb(&artifact);
    assert_eq!(entry.coordinates(), Some(published));

    let json = serde_json::to_string(&SourceLabel::Geocoder).expect("geocoder serializes");
    assert_eq!(json, "\"Geocoder\"");
    assert_eq!(
        serde_json::from_str::<SourceLabel>(&json).expect("geocoder parses"),
        SourceLabel::Geocoder
    );
}
