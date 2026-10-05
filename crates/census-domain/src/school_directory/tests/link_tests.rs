use crate::school_directory::link::{
    AttestedRecord, DirectoryIndex, LinkDecision, LinkRule, ReviewReason,
};
use crate::school_directory::{
    AssociationLabel, CityName, Grade, GradeSpan, IdentifiedKey, NcesSchoolId, NumberedGrade,
    PostalAddress, PssId, SchoolDirectoryEntry, SchoolName, SourceLabel, StateRecordId, StreetLine,
    Website, ZipCode,
};
use crate::UsJurisdiction;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn span(low: u8, high: u8) -> Result<GradeSpan, Box<dyn std::error::Error>> {
    Ok(GradeSpan::new(
        Grade::Numbered(NumberedGrade::new(low)?),
        Grade::Numbered(NumberedGrade::new(high)?),
    )?)
}

fn address(
    line1: Option<&str>,
    city: &str,
    state: UsJurisdiction,
) -> Result<PostalAddress, Box<dyn std::error::Error>> {
    let line1 = match line1 {
        Some(line1) => Some(StreetLine::parse(line1)?),
        None => None,
    };
    PostalAddress::of(
        line1,
        None,
        Some(CityName::parse(city)?),
        Some(state),
        Some(ZipCode::parse("43000")?),
    )
    .ok_or("address is empty")
    .map_err(Into::into)
}

fn nces_entry(
    id: &str,
    name: &str,
    city: &str,
    state: UsJurisdiction,
    line1: &str,
    grades: (u8, u8),
) -> Result<SchoolDirectoryEntry, Box<dyn std::error::Error>> {
    Ok(SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse(id)?),
        SourceLabel::Ccd,
        Some(SchoolName::parse(name)?),
    )
    .with_address(Some(address(Some(line1), city, state)?))
    .with_grades(Some(span(grades.0, grades.1)?)))
}

#[test]
fn exact_name_city_state_links() -> TestResult {
    let entry = nces_entry(
        "390000000001",
        "Springfield High School",
        "Springfield",
        UsJurisdiction::Ohio,
        "1 Main St",
        (9, 12),
    )?
    .with_website(Some(
        Website::parse("https://springfield.example/high")?.ok_or("website is empty")?,
    ));
    let index = DirectoryIndex::build(&[entry]);
    let decision = index.link(
        "Springfield High School",
        "springfield high school",
        Some("Springfield"),
        UsJurisdiction::Ohio,
        &[],
        &[],
    );
    let LinkDecision::Linked(hit) = decision else {
        return Err("expected a link".into());
    };
    check!(eq; hit.rule, LinkRule::ExactName);
    check!(eq;
        hit.key,
        IdentifiedKey::Nces(NcesSchoolId::parse("390000000001")?)
    );
    check!(eq;
        hit.address.as_ref().and_then(PostalAddress::line1).map(StreetLine::as_str),
        Some("1 Main St")
    );
    check!(eq; hit.website.as_deref(), Some("https://springfield.example/high"));
    check!(eq; hit.source, SourceLabel::Ccd);
    Ok(())
}

#[test]
fn suffixed_corpus_name_links_through_its_core() -> TestResult {
    let index = DirectoryIndex::build(&[nces_entry(
        "390000000002",
        "New Bloomfield High School",
        "New Bloomfield",
        UsJurisdiction::Missouri,
        "307 Redwood Dr",
        (9, 12),
    )?]);
    let decision = index.link(
        "New Bloomfield",
        "new bloomfield",
        Some("New Bloomfield"),
        UsJurisdiction::Missouri,
        &[],
        &[],
    );
    let LinkDecision::Linked(hit) = decision else {
        return Err("expected a link".into());
    };
    check!(eq; hit.matched_form, "new bloomfield");
    Ok(())
}

#[test]
fn census_core_form_links_suffixed_name() -> TestResult {
    let index = DirectoryIndex::build(&[nces_entry(
        "390000000003",
        "Springfield Local High School",
        "Springfield",
        UsJurisdiction::Ohio,
        "2 Main St",
        (9, 12),
    )?]);
    let decision = index.link(
        "Springfield Local School",
        "springfield local school",
        Some("Springfield"),
        UsJurisdiction::Ohio,
        &[],
        &[],
    );
    let LinkDecision::Linked(hit) = decision else {
        return Err("expected a link".into());
    };
    check!(eq; hit.rule, LinkRule::CoreName);
    check!(eq; hit.matched_form, "springfield local");
    Ok(())
}

#[test]
fn middle_entries_are_excluded_for_high_names() -> TestResult {
    let index = DirectoryIndex::build(&[nces_entry(
        "390000000004",
        "Summit Middle School",
        "Frisco",
        UsJurisdiction::Colorado,
        "158 School Road",
        (6, 8),
    )?]);
    let decision = index.link(
        "Summit High School",
        "summit high school",
        Some("Frisco"),
        UsJurisdiction::Colorado,
        &[],
        &[],
    );
    check!(eq; decision, LinkDecision::NoMatch);
    Ok(())
}

#[test]
fn junior_high_names_link_to_middle_entries() -> TestResult {
    let middle = nces_entry(
        "390000000005",
        "Magnolia Junior High School",
        "Magnolia",
        UsJurisdiction::Arkansas,
        "3 Middle St",
        (7, 9),
    )?;
    let high = nces_entry(
        "390000000006",
        "Magnolia High School",
        "Magnolia",
        UsJurisdiction::Arkansas,
        "1400 High School Drive",
        (9, 12),
    )?;
    let index = DirectoryIndex::build(&[middle, high]);
    let decision = index.link(
        "Magnolia Junior High",
        "magnolia junior high",
        Some("Magnolia"),
        UsJurisdiction::Arkansas,
        &[],
        &[],
    );
    let LinkDecision::Linked(hit) = decision else {
        return Err("expected a link".into());
    };
    check!(eq;
        hit.key,
        IdentifiedKey::Nces(NcesSchoolId::parse("390000000005")?)
    );
    check!(eq; hit.rule, LinkRule::CoreName);
    Ok(())
}

#[test]
fn competing_keys_set_an_ambiguous_review() -> TestResult {
    let first = nces_entry(
        "390000000007",
        "Central High School",
        "Springfield",
        UsJurisdiction::Ohio,
        "4 Main St",
        (9, 12),
    )?;
    let second = nces_entry(
        "390000000008",
        "Central High School",
        "Springfield",
        UsJurisdiction::Ohio,
        "5 Main St",
        (9, 12),
    )?;
    let index = DirectoryIndex::build(&[first, second]);
    let decision = index.link(
        "Central High School",
        "central high school",
        Some("Springfield"),
        UsJurisdiction::Ohio,
        &[],
        &[],
    );
    let LinkDecision::Review { reason, candidates } = decision else {
        return Err("expected a review".into());
    };
    check!(eq; reason, ReviewReason::Ambiguous);
    check!(eq; candidates.len(), 2);
    Ok(())
}

#[test]
fn parenthetical_head_links() -> TestResult {
    let index = DirectoryIndex::build(&[nces_entry(
        "390000000009",
        "Morro Bay High School",
        "Morro Bay",
        UsJurisdiction::California,
        "235 Atascadero Rd",
        (9, 12),
    )?]);
    let decision = index.link(
        "Morro Bay (CS)",
        "morro bay cs",
        Some("Morro Bay"),
        UsJurisdiction::California,
        &[],
        &[],
    );
    let LinkDecision::Linked(hit) = decision else {
        return Err("expected a link".into());
    };
    check!(eq; hit.rule, LinkRule::Parenthetical);
    Ok(())
}

#[test]
fn parenthetical_inner_links() -> TestResult {
    let index = DirectoryIndex::build(&[nces_entry(
        "390000000010",
        "St. Teresa High School",
        "Decatur",
        UsJurisdiction::Illinois,
        "2710 N Water St",
        (9, 12),
    )?]);
    let decision = index.link(
        "Decatur (St. Teresa)",
        "decatur st teresa",
        Some("Decatur"),
        UsJurisdiction::Illinois,
        &[],
        &[],
    );
    let LinkDecision::Linked(hit) = decision else {
        return Err("expected a link".into());
    };
    check!(eq; hit.rule, LinkRule::ParentheticalInner);
    Ok(())
}

#[test]
fn alias_links() -> TestResult {
    let index = DirectoryIndex::build(&[nces_entry(
        "390000000011",
        "William Penn Charter School",
        "Philadelphia",
        UsJurisdiction::Pennsylvania,
        "3000 W School House Ln",
        (9, 12),
    )?]);
    let decision = index.link(
        "Penn Charter",
        "penn charter",
        Some("Philadelphia"),
        UsJurisdiction::Pennsylvania,
        &["William Penn Charter School".to_string()],
        &[],
    );
    let LinkDecision::Linked(hit) = decision else {
        return Err("expected a link".into());
    };
    check!(eq; hit.rule, LinkRule::Alias);
    Ok(())
}

#[test]
fn a_state_unique_name_links_across_city_variants() -> TestResult {
    let index = DirectoryIndex::build(&[nces_entry(
        "390000000012",
        "Rochester High School",
        "Rochester Hills",
        UsJurisdiction::Michigan,
        "180 S Livernois Rd",
        (9, 12),
    )?]);
    let decision = index.link(
        "Rochester",
        "rochester",
        Some("Rochester"),
        UsJurisdiction::Michigan,
        &[],
        &[],
    );
    let LinkDecision::Linked(hit) = decision else {
        return Err("expected a link".into());
    };
    check!(eq;
        hit.key,
        IdentifiedKey::Nces(NcesSchoolId::parse("390000000012")?)
    );
    check!(eq;
        hit.address.as_ref().and_then(PostalAddress::city).map(CityName::as_str),
        Some("Rochester Hills")
    );
    Ok(())
}

#[test]
fn city_agreement_resolves_same_named_state_entries() -> TestResult {
    let index = DirectoryIndex::build(&[
        nces_entry(
            "390000000012",
            "Central High School",
            "Grand Rapids",
            UsJurisdiction::Michigan,
            "1 Grand Ave",
            (9, 12),
        )?,
        nces_entry(
            "390000000013",
            "Central High School",
            "Battle Creek",
            UsJurisdiction::Michigan,
            "2 Battle Ave",
            (9, 12),
        )?,
    ]);
    let decision = index.link(
        "Central High School",
        "central high school",
        Some("Battle Creek"),
        UsJurisdiction::Michigan,
        &[],
        &[],
    );
    let LinkDecision::Linked(hit) = decision else {
        return Err("expected a link".into());
    };
    check!(eq;
        hit.key,
        IdentifiedKey::Nces(NcesSchoolId::parse("390000000013")?)
    );
    Ok(())
}

#[test]
fn abbreviations_link_against_spelled_names() -> TestResult {
    for (census, corpus) in [
        ("Alvin H S", "Alvin High School"),
        ("Midland MHS", "Midland High School"),
        ("Springfield High School - 2", "Springfield High School"),
    ] {
        let index = DirectoryIndex::build(&[nces_entry(
            "390000000014",
            corpus,
            "Springfield",
            UsJurisdiction::Ohio,
            "3 Spelled Ave",
            (9, 12),
        )?]);
        let decision = index.link(
            census,
            &census.to_lowercase(),
            Some("Springfield"),
            UsJurisdiction::Ohio,
            &[],
            &[],
        );
        let LinkDecision::Linked(hit) = decision else {
            return Err(format!("expected a link for {census}").into());
        };
        check!(eq;
            hit.key,
            IdentifiedKey::Nces(NcesSchoolId::parse("390000000014")?)
        );
    }
    Ok(())
}

#[test]
fn junior_high_abbreviations_link_to_middle_entries() -> TestResult {
    let index = DirectoryIndex::build(&[nces_entry(
        "390000000015",
        "Magnolia Junior High School",
        "Magnolia",
        UsJurisdiction::Ohio,
        "4 Middle Ave",
        (7, 9),
    )?]);
    let decision = index.link(
        "Magnolia JHS",
        "magnolia jhs",
        Some("Magnolia"),
        UsJurisdiction::Ohio,
        &[],
        &[],
    );
    let LinkDecision::Linked(hit) = decision else {
        return Err("expected a link".into());
    };
    check!(eq;
        hit.key,
        IdentifiedKey::Nces(NcesSchoolId::parse("390000000015")?)
    );
    check!(eq; hit.rule, LinkRule::CoreName);
    Ok(())
}

#[test]
fn structural_words_link_across_reordered_names() -> TestResult {
    let index = DirectoryIndex::build(&[nces_entry(
        "390000000016",
        "Notre Dame Academy",
        "Toledo",
        UsJurisdiction::Ohio,
        "5 Academy Way",
        (9, 12),
    )?]);
    let decision = index.link(
        "Academy of Notre Dame",
        "academy of notre dame",
        Some("Toledo"),
        UsJurisdiction::Ohio,
        &[],
        &[],
    );
    let LinkDecision::Linked(hit) = decision else {
        return Err("expected a link".into());
    };
    check!(eq;
        hit.key,
        IdentifiedKey::Nces(NcesSchoolId::parse("390000000016")?)
    );
    Ok(())
}

#[test]
fn pss_entries_keep_their_source() -> TestResult {
    let entry = SchoolDirectoryEntry::identified(
        IdentifiedKey::Pss(PssId::parse("12345678")?),
        SourceLabel::Pss,
        Some(SchoolName::parse("Marianapolis Preparatory School")?),
    )
    .with_address(Some(address(
        Some("26 Chase Rd"),
        "Thompson",
        UsJurisdiction::Connecticut,
    )?));
    let index = DirectoryIndex::build(&[entry]);
    let decision = index.link(
        "Marianapolis Preparatory School",
        "marianapolis preparatory school",
        Some("Thompson"),
        UsJurisdiction::Connecticut,
        &[],
        &[],
    );
    let LinkDecision::Linked(hit) = decision else {
        return Err("expected a link".into());
    };
    check!(eq; hit.source, SourceLabel::Pss);
    check!(eq; hit.key, IdentifiedKey::Pss(PssId::parse("12345678")?));
    Ok(())
}

#[test]
fn foreign_state_entries_are_not_candidates() -> TestResult {
    let index = DirectoryIndex::build(&[nces_entry(
        "390000000013",
        "Springfield High School",
        "Springfield",
        UsJurisdiction::Indiana,
        "6 Main St",
        (9, 12),
    )?]);
    let decision = index.link(
        "Springfield High School",
        "springfield high school",
        Some("Springfield"),
        UsJurisdiction::Ohio,
        &[],
        &[],
    );
    check!(eq; decision, LinkDecision::NoMatch);
    Ok(())
}

#[test]
fn entries_without_street_are_skipped() -> TestResult {
    let entry = SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse("390000000014")?),
        SourceLabel::Ccd,
        Some(SchoolName::parse("Springfield High School")?),
    )
    .with_address(Some(address(None, "Springfield", UsJurisdiction::Ohio)?));
    let index = DirectoryIndex::build(&[entry]);
    let decision = index.link(
        "Springfield High School",
        "springfield high school",
        Some("Springfield"),
        UsJurisdiction::Ohio,
        &[],
        &[],
    );
    check!(eq; decision, LinkDecision::NoMatch);
    Ok(())
}

fn state_record_entry(
    id: &str,
    name: &str,
    city: &str,
    state: UsJurisdiction,
    line1: Option<&str>,
    label: SourceLabel,
) -> Result<SchoolDirectoryEntry, Box<dyn std::error::Error>> {
    Ok(SchoolDirectoryEntry::identified(
        IdentifiedKey::StateRecord {
            state,
            id: StateRecordId::parse(id)?,
        },
        label,
        Some(SchoolName::parse(name)?),
    )
    .with_address(Some(address(line1, city, state)?))
    .with_grades(Some(span(9, 12)?)))
}

fn association(state: UsJurisdiction) -> SourceLabel {
    SourceLabel::AthleticAssociation { state }
}

#[test]
fn a_city_only_association_entry_links_on_name_and_city() -> TestResult {
    let index = DirectoryIndex::build(&[state_record_entry(
        "3",
        "Alcoa High School",
        "Alcoa",
        UsJurisdiction::Tennessee,
        None,
        association(UsJurisdiction::Tennessee),
    )?]);
    let decision = index.link(
        "Alcoa High School",
        "alcoa high school",
        Some("Alcoa"),
        UsJurisdiction::Tennessee,
        &[],
        &[],
    );
    let LinkDecision::Linked(hit) = decision else {
        return Err("expected a link".into());
    };
    check!(eq; hit.rule, LinkRule::ExactName);
    check!(eq;
        hit.key,
        IdentifiedKey::StateRecord {
            state: UsJurisdiction::Tennessee,
            id: StateRecordId::parse("3")?
        }
    );
    check!(eq; hit.source, association(UsJurisdiction::Tennessee));
    check!(eq; hit.address, None);
    Ok(())
}

#[test]
fn a_city_only_association_entry_never_links_across_cities() -> TestResult {
    let index = DirectoryIndex::build(&[state_record_entry(
        "3",
        "Alcoa High School",
        "Alcoa",
        UsJurisdiction::Tennessee,
        None,
        association(UsJurisdiction::Tennessee),
    )?]);
    let decision = index.link(
        "Alcoa High School",
        "alcoa high school",
        Some("Knoxville"),
        UsJurisdiction::Tennessee,
        &[],
        &[],
    );
    check!(eq; decision, LinkDecision::NoMatch);
    Ok(())
}

#[test]
fn an_attested_association_record_links_across_cities() -> TestResult {
    let index = DirectoryIndex::build(&[state_record_entry(
        "3",
        "Alcoa High School",
        "Alcoa",
        UsJurisdiction::Tennessee,
        None,
        association(UsJurisdiction::Tennessee),
    )?]);
    let attested = [AttestedRecord {
        label: association(UsJurisdiction::Tennessee),
        id: StateRecordId::parse("3")?,
    }];
    let decision = index.link(
        "Alcoa High School",
        "alcoa high school",
        Some("Knoxville"),
        UsJurisdiction::Tennessee,
        &[],
        &attested,
    );
    let LinkDecision::Linked(hit) = decision else {
        return Err("expected a link".into());
    };
    check!(eq;
        hit.key,
        IdentifiedKey::StateRecord {
            state: UsJurisdiction::Tennessee,
            id: StateRecordId::parse("3")?
        }
    );
    Ok(())
}

#[test]
fn an_attestation_under_another_label_never_links_across_cities() -> TestResult {
    let index = DirectoryIndex::build(&[state_record_entry(
        "3",
        "Alcoa High School",
        "Alcoa",
        UsJurisdiction::Tennessee,
        None,
        association(UsJurisdiction::Tennessee),
    )?]);
    let attested = [AttestedRecord {
        label: SourceLabel::StateEducationAgency {
            state: UsJurisdiction::Tennessee,
        },
        id: StateRecordId::parse("3")?,
    }];
    let decision = index.link(
        "Alcoa High School",
        "alcoa high school",
        Some("Knoxville"),
        UsJurisdiction::Tennessee,
        &[],
        &attested,
    );
    check!(eq; decision, LinkDecision::NoMatch);
    Ok(())
}

#[test]
fn association_entries_in_another_state_never_link() -> TestResult {
    let index = DirectoryIndex::build(&[state_record_entry(
        "3",
        "Alcoa High School",
        "Alcoa",
        UsJurisdiction::Tennessee,
        None,
        association(UsJurisdiction::Tennessee),
    )?]);
    let decision = index.link(
        "Alcoa High School",
        "alcoa high school",
        Some("Alcoa"),
        UsJurisdiction::Ohio,
        &[],
        &[],
    );
    check!(eq; decision, LinkDecision::NoMatch);
    Ok(())
}

#[test]
fn association_labels_in_another_state_never_link() -> TestResult {
    let index = DirectoryIndex::build(&[state_record_entry(
        "3",
        "Alcoa High School",
        "Alcoa",
        UsJurisdiction::Tennessee,
        None,
        association(UsJurisdiction::Georgia),
    )?]);
    let decision = index.link(
        "Alcoa High School",
        "alcoa high school",
        Some("Alcoa"),
        UsJurisdiction::Tennessee,
        &[],
        &[],
    );
    check!(eq; decision, LinkDecision::NoMatch);
    Ok(())
}

#[test]
fn state_record_entries_without_an_association_label_stay_unindexed() -> TestResult {
    let index = DirectoryIndex::build(&[state_record_entry(
        "3",
        "Alcoa High School",
        "Alcoa",
        UsJurisdiction::Tennessee,
        None,
        SourceLabel::Ccd,
    )?]);
    let decision = index.link(
        "Alcoa High School",
        "alcoa high school",
        Some("Alcoa"),
        UsJurisdiction::Tennessee,
        &[],
        &[],
    );
    check!(eq; decision, LinkDecision::NoMatch);
    Ok(())
}

#[test]
fn state_record_entries_whose_address_contradicts_the_record_state_stay_unindexed() -> TestResult {
    let entry = SchoolDirectoryEntry::identified(
        IdentifiedKey::StateRecord {
            state: UsJurisdiction::Tennessee,
            id: StateRecordId::parse("3")?,
        },
        association(UsJurisdiction::Tennessee),
        Some(SchoolName::parse("Alcoa High School")?),
    )
    .with_address(Some(address(None, "Alcoa", UsJurisdiction::Georgia)?));
    let index = DirectoryIndex::build(&[entry]);
    let decision = index.link(
        "Alcoa High School",
        "alcoa high school",
        Some("Alcoa"),
        UsJurisdiction::Tennessee,
        &[],
        &[],
    );
    check!(eq; decision, LinkDecision::NoMatch);
    Ok(())
}

#[test]
fn state_education_records_link_on_name_and_city() -> TestResult {
    let index = DirectoryIndex::build(&[state_record_entry(
        "441001",
        "Rochester High School",
        "Rochester",
        UsJurisdiction::NewYork,
        Some("180 S Livernois Rd"),
        SourceLabel::StateEducationAgency {
            state: UsJurisdiction::NewYork,
        },
    )?]);
    let decision = index.link(
        "Rochester High School",
        "rochester high school",
        Some("Rochester"),
        UsJurisdiction::NewYork,
        &[],
        &[],
    );
    let LinkDecision::Linked(hit) = decision else {
        return Err("expected a link".into());
    };
    check!(eq; hit.source, SourceLabel::StateEducationAgency { state: UsJurisdiction::NewYork });
    check!(eq; hit.address.as_ref().and_then(PostalAddress::line1).map(StreetLine::as_str), Some("180 S Livernois Rd"));
    Ok(())
}

#[test]
fn a_street_bearing_association_entry_keeps_its_address() -> TestResult {
    let index = DirectoryIndex::build(&[state_record_entry(
        "3",
        "Alcoa High School",
        "Alcoa",
        UsJurisdiction::Tennessee,
        Some("1 Tornado Trail"),
        association(UsJurisdiction::Tennessee),
    )?]);
    let decision = index.link(
        "Alcoa High School",
        "alcoa high school",
        Some("Alcoa"),
        UsJurisdiction::Tennessee,
        &[],
        &[],
    );
    let LinkDecision::Linked(hit) = decision else {
        return Err("expected a link".into());
    };
    check!(eq;
        hit.address.as_ref().and_then(PostalAddress::line1).map(StreetLine::as_str),
        Some("1 Tornado Trail")
    );
    Ok(())
}

#[test]
fn a_private_association_record_still_needs_city_agreement() -> TestResult {
    let index = DirectoryIndex::build(&[state_record_entry(
        "3",
        "Alcoa High School",
        "Alcoa",
        UsJurisdiction::Tennessee,
        None,
        SourceLabel::PrivateAssociation {
            label: AssociationLabel::parse("nais")?,
        },
    )?]);
    let decision = index.link(
        "Alcoa High School",
        "alcoa high school",
        Some("Knoxville"),
        UsJurisdiction::Tennessee,
        &[],
        &[],
    );
    check!(eq; decision, LinkDecision::NoMatch);
    Ok(())
}

#[test]
fn a_designated_campus_never_links_the_undesignated_campus() -> TestResult {
    let index = DirectoryIndex::build(&[nces_entry(
        "390000000101",
        "Lincoln High School",
        "Lincoln",
        UsJurisdiction::Tennessee,
        "1 Main St",
        (9, 12),
    )?]);
    let decision = index.link(
        "Lincoln High School (East Campus)",
        "lincoln high school east campus",
        Some("Lincoln"),
        UsJurisdiction::Tennessee,
        &[],
        &[],
    );
    check!(eq; decision, LinkDecision::NoMatch);
    Ok(())
}

#[test]
fn a_campus_pair_keeps_its_own_key_through_the_index() -> TestResult {
    let index = DirectoryIndex::build(&[
        nces_entry(
            "390000000102",
            "Lincoln High School",
            "Lincoln",
            UsJurisdiction::Tennessee,
            "1 Main St",
            (9, 12),
        )?,
        nces_entry(
            "390000000103",
            "Lincoln High School (East)",
            "Lincoln",
            UsJurisdiction::Tennessee,
            "2 East St",
            (9, 12),
        )?,
    ]);
    let main = index.link(
        "Lincoln High School",
        "lincoln high school",
        Some("Lincoln"),
        UsJurisdiction::Tennessee,
        &[],
        &[],
    );
    let east = index.link(
        "Lincoln High School (East)",
        "lincoln high school east",
        Some("Lincoln"),
        UsJurisdiction::Tennessee,
        &[],
        &[],
    );
    let (LinkDecision::Linked(main), LinkDecision::Linked(east)) = (main, east) else {
        return Err("expected both campuses to link".into());
    };
    check!(eq;
        main.key,
        IdentifiedKey::Nces(NcesSchoolId::parse("390000000102")?)
    );
    check!(eq;
        east.key,
        IdentifiedKey::Nces(NcesSchoolId::parse("390000000103")?)
    );
    check!(eq; east.rule, LinkRule::ExactName);
    Ok(())
}
