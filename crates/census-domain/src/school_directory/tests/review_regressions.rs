use crate::school_directory::{
    AssociationLabel, DirectoryError, Grade, IdentifiedKey, NcesSchoolId, NumberedGrade, Phone,
    SchoolDirectoryEntry, SchoolName, SourceLabel,
};
use crate::UsJurisdiction;

fn phone_entry(source: SourceLabel, phone: &str) -> SchoolDirectoryEntry {
    SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse("010000500870").expect("identifier")),
        source,
        None,
    )
    .with_phone(Phone::parse(phone).expect("phone"))
}

#[test]
fn field_precedence_survives_gap_filling_and_persisted_round_trips() {
    let entries = [
        phone_entry(SourceLabel::Ccd, ""),
        phone_entry(
            SourceLabel::AthleticAssociation {
                state: UsJurisdiction::Alabama,
            },
            "9999999999",
        ),
        phone_entry(
            SourceLabel::StateEducationAgency {
                state: UsJurisdiction::Alabama,
            },
            "1111111111",
        ),
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
        let bytes = serde_json::to_vec(&merged).expect("serialize intermediate merge");
        merged = serde_json::from_slice(&bytes).expect("read intermediate merge");
        merged.absorb(&entries[order[2]]);
        println!(
            "order={order:?} merged phone={:?}",
            merged.phone().map(Phone::as_str)
        );
        assert_eq!(merged.phone().map(Phone::as_str), Some("1111111111"));
    }
}

#[test]
fn baselines_reject_missing_or_inconsistent_provenance() {
    let entry = phone_entry(SourceLabel::Ccd, "1111111111");
    let mut value = serde_json::to_value(&entry).expect("entry json");
    value
        .as_object_mut()
        .expect("entry object")
        .remove("provenance");
    let error = serde_json::from_value::<SchoolDirectoryEntry>(value).expect_err("old baseline");
    assert_eq!(error.to_string(), "missing field `provenance`");
    let mut value = serde_json::to_value(&entry).expect("entry json");
    value["provenance"] = serde_json::json!({});
    let error = serde_json::from_value::<SchoolDirectoryEntry>(value).expect_err("no field source");
    assert_eq!(error.to_string(), "field phone is missing provenance");
    let mut value = serde_json::to_value(&entry).expect("entry json");
    value["provenance"]["Name"] = serde_json::json!("Ccd");
    let error =
        serde_json::from_value::<SchoolDirectoryEntry>(value).expect_err("absent field source");
    assert_eq!(error.to_string(), "absent field name has provenance");
    let mut value = serde_json::to_value(&entry).expect("entry json");
    value["provenance"]["Phone"] = serde_json::json!("Pss");
    let error =
        serde_json::from_value::<SchoolDirectoryEntry>(value).expect_err("unrecorded source");
    assert_eq!(
        error.to_string(),
        "field phone provenance names an unrecorded source"
    );
    let entry = phone_entry(
        SourceLabel::StateEducationAgency {
            state: UsJurisdiction::Alabama,
        },
        "1111111111",
    );
    let mut value = serde_json::to_value(&entry).expect("entry json");
    value["provenance"]["Phone"]["StateEducationAgency"]["rank"] = serde_json::json!(4);
    let error =
        serde_json::from_value::<SchoolDirectoryEntry>(value).expect_err("forged source rank");
    assert_eq!(error.to_string(), "unknown field `rank`, expected `state`");
}

#[test]
fn numbered_grades_keep_valid_wire_bytes_and_reject_out_of_range_values() {
    for number in 1..=12 {
        let grade = Grade::Numbered(NumberedGrade::try_from(number).expect("admitted grade"));
        let json = format!(r#"{{"Numbered":{number}}}"#);
        assert_eq!(serde_json::to_string(&grade).expect("serialize"), json);
        assert_eq!(
            serde_json::from_str::<Grade>(&json).expect("deserialize"),
            grade
        );
    }
    for number in [0, 13, u8::MAX] {
        assert_eq!(
            NumberedGrade::new(number),
            Err(DirectoryError::UnsupportedGrade {
                value: number.to_string()
            })
        );
        let json = format!(r#"{{"Numbered":{number}}}"#);
        let error = serde_json::from_str::<Grade>(&json).expect_err("invalid grade");
        println!("grade {number} rejected: {error}");
        assert!(error.to_string().contains("is not a supported grade"));
    }
}

#[test]
fn school_name_deserialization_rejects_invalid_or_noncanonical_artifacts() {
    for raw in ["", "  ", " A ", "A  B", "A\nB"] {
        let json = serde_json::to_string(raw).expect("json input");
        let error = serde_json::from_str::<SchoolName>(&json).expect_err("invalid name");
        println!("name {raw:?} rejected: {error}");
    }
    assert_eq!(
        SchoolName::try_from(String::new()),
        Err(DirectoryError::EmptyField {
            field: "school name"
        })
    );
    for raw in [
        "Albertville High School",
        "École du Lac",
        "A A KINGSTON MIDDLE SCHOOL",
    ] {
        let name = SchoolName::parse(raw).expect("name");
        let json = serde_json::to_string(raw).expect("previous wire form");
        assert_eq!(serde_json::to_string(&name).expect("serialize"), json);
        assert_eq!(
            serde_json::from_str::<SchoolName>(&json).expect("deserialize"),
            name
        );
    }
}

#[test]
fn identical_values_from_equal_rank_sources_have_order_independent_provenance() {
    let row = |label: &str| {
        phone_entry(
            SourceLabel::PrivateAssociation {
                label: AssociationLabel::parse(label).expect("association"),
            },
            "1111111111",
        )
    };
    let nais = row("NAIS");
    let cape = row("CAPE");
    let mut first = nais.clone();
    first.absorb(&cape);
    let mut reversed = cape;
    reversed.absorb(&nais);
    let first_bytes = serde_json::to_vec(&first).expect("forward json");
    let reversed_bytes = serde_json::to_vec(&reversed).expect("reverse json");
    assert_eq!(first.phone().map(Phone::as_str), Some("1111111111"));
    assert_eq!(first_bytes, reversed_bytes);
    println!("equal-rank source reversal preserves identical artifact bytes");
}
