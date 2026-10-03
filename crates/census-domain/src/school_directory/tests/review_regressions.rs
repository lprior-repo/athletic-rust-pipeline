use super::TestResult;
use crate::school_directory::{
    AssociationLabel, DirectoryError, Grade, IdentifiedKey, NcesSchoolId, NumberedGrade, Phone,
    SchoolDirectoryEntry, SchoolName, SourceLabel,
};
use crate::UsJurisdiction;

fn phone_entry(
    source: SourceLabel,
    phone: &str,
) -> Result<SchoolDirectoryEntry, Box<dyn std::error::Error>> {
    Ok(SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse("010000500870")?),
        source,
        None,
    )
    .with_phone(Phone::parse(phone)?))
}

#[test]
fn field_precedence_survives_gap_filling_and_persisted_round_trips() -> TestResult {
    let entries = [
        phone_entry(SourceLabel::Ccd, "")?,
        phone_entry(
            SourceLabel::AthleticAssociation {
                state: UsJurisdiction::Alabama,
            },
            "9999999999",
        )?,
        phone_entry(
            SourceLabel::StateEducationAgency {
                state: UsJurisdiction::Alabama,
            },
            "1111111111",
        )?,
    ];
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let mut merged = entries[order[0]].clone();
        merged.absorb(&entries[order[1]]);
        let bytes = serde_json::to_vec(&merged)?;
        merged = serde_json::from_slice(&bytes)?;
        merged.absorb(&entries[order[2]]);
        println!(
            "order={order:?} merged phone={:?}",
            merged.phone().map(Phone::as_str)
        );
        check!(eq; merged.phone().map(Phone::as_str), Some("1111111111"));
    }
    Ok(())
}

#[test]
fn baselines_reject_missing_or_inconsistent_provenance() -> TestResult {
    let entry = phone_entry(SourceLabel::Ccd, "1111111111")?;
    let mut value = serde_json::to_value(&entry)?;
    value
        .as_object_mut()
        .ok_or("entry object")?
        .remove("provenance");
    match serde_json::from_value::<SchoolDirectoryEntry>(value) {
        Err(_) => {}
        Ok(entry) => return Err(format!("old baseline was accepted: {entry:?}").into()),
    };
    let mut value = serde_json::to_value(&entry)?;
    value["provenance"] = serde_json::json!({});
    match serde_json::from_value::<SchoolDirectoryEntry>(value) {
        Err(_) => {}
        Ok(entry) => return Err(format!("missing field source was accepted: {entry:?}").into()),
    };
    let mut value = serde_json::to_value(&entry)?;
    value["provenance"]["Name"] = serde_json::json!("Ccd");
    match serde_json::from_value::<SchoolDirectoryEntry>(value) {
        Err(_) => {}
        Ok(entry) => return Err(format!("absent field source was accepted: {entry:?}").into()),
    };
    let mut value = serde_json::to_value(&entry)?;
    value["provenance"]["Phone"] = serde_json::json!("Pss");
    match serde_json::from_value::<SchoolDirectoryEntry>(value) {
        Err(_) => {}
        Ok(entry) => return Err(format!("unrecorded source was accepted: {entry:?}").into()),
    };
    let entry = phone_entry(
        SourceLabel::StateEducationAgency {
            state: UsJurisdiction::Alabama,
        },
        "1111111111",
    )?;
    let mut value = serde_json::to_value(&entry)?;
    value["provenance"]["Phone"]["StateEducationAgency"]["rank"] = serde_json::json!(4);
    match serde_json::from_value::<SchoolDirectoryEntry>(value) {
        Err(_) => {}
        Ok(entry) => return Err(format!("forged source rank was accepted: {entry:?}").into()),
    };
    Ok(())
}

#[test]
fn numbered_grades_keep_valid_wire_bytes_and_reject_out_of_range_values() -> TestResult {
    for number in 1..=12 {
        let grade = Grade::Numbered(NumberedGrade::try_from(number)?);
        let json = format!(r#"{{"Numbered":{number}}}"#);
        check!(eq; serde_json::to_string(&grade)?, json);
        check!(eq; serde_json::from_str::<Grade>(&json)?, grade);
    }
    for number in [0, 13, u8::MAX] {
        check!(eq; NumberedGrade::new(number),
        Err(DirectoryError::UnsupportedGrade {
            value: number.to_string()
        }));
        let json = format!(r#"{{"Numbered":{number}}}"#);
        let error = match serde_json::from_str::<Grade>(&json) {
            Err(error) => error,
            Ok(grade) => return Err(format!("invalid grade was accepted: {grade:?}").into()),
        };
        println!("grade {number} rejected: {error}");
    }
    Ok(())
}

#[test]
fn school_name_deserialization_rejects_invalid_or_noncanonical_artifacts() -> TestResult {
    for raw in ["", "  ", " A ", "A  B", "A\nB"] {
        let json = serde_json::to_string(raw)?;
        let error = match serde_json::from_str::<SchoolName>(&json) {
            Err(error) => error,
            Ok(name) => return Err(format!("invalid name was accepted: {name:?}").into()),
        };
        println!("name {raw:?} rejected: {error}");
    }
    check!(eq; SchoolName::try_from(String::new()),
    Err(DirectoryError::EmptyField {
        field: "school name"
    }));
    for raw in [
        "Albertville High School",
        "École du Lac",
        "A A KINGSTON MIDDLE SCHOOL",
    ] {
        let name = SchoolName::parse(raw)?;
        let json = serde_json::to_string(raw)?;
        check!(eq; serde_json::to_string(&name)?, json);
        check!(eq; serde_json::from_str::<SchoolName>(&json)?, name);
    }
    Ok(())
}

#[test]
fn identical_values_from_equal_rank_sources_have_order_independent_provenance() -> TestResult {
    let row = |label: &str| -> Result<SchoolDirectoryEntry, Box<dyn std::error::Error>> {
        phone_entry(
            SourceLabel::PrivateAssociation {
                label: AssociationLabel::parse(label)?,
            },
            "1111111111",
        )
    };
    let nais = row("NAIS")?;
    let cape = row("CAPE")?;
    let mut first = nais.clone();
    first.absorb(&cape);
    let mut reversed = cape;
    reversed.absorb(&nais);
    let first_bytes = serde_json::to_vec(&first)?;
    let reversed_bytes = serde_json::to_vec(&reversed)?;
    check!(eq; first.phone().map(Phone::as_str), Some("1111111111"));
    check!(eq; first_bytes, reversed_bytes);
    println!("equal-rank source reversal preserves identical artifact bytes");
    Ok(())
}
