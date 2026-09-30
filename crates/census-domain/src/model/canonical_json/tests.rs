use super::serialized_bytes;
use super::serialized_digest;
use crate::model::{
    athlete_identity_digest, identity_verdict_digest, CanonicalAthlete, Gender, GradYear,
    ReviewCase, ReviewVerdictRecord, SchoolId, SourceAthleteObservation, SourceIdentity,
    SourceNamespace, SourceObservation, SourceSchoolObservation,
};
use serde::{Serialize, Serializer};
use std::collections::BTreeMap;

fn assert_matches<T: Serialize + ?Sized>(value: &T) {
    let canonical = serialized_bytes(value).expect("canonical json write");
    let reference = serde_json::to_vec(value).expect("serde json write");
    assert_eq!(canonical, reference);
}

#[derive(Serialize)]
struct Inner {
    a: u32,
    b: Option<String>,
    c: Vec<i16>,
}

#[derive(Serialize)]
struct Newtype(u64);

#[derive(Serialize)]
struct TupleStruct(bool, char);

#[derive(Serialize)]
struct UnitStruct;

#[derive(Serialize)]
struct Skipped {
    present: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    absent: Option<u8>,
}

#[derive(Serialize)]
struct Flattened {
    id: u32,
    #[serde(flatten)]
    extra: BTreeMap<String, i64>,
}

#[derive(Serialize)]
enum Shape {
    Unit,
    Newtype(u8),
    Tuple(u8, u8),
    Struct { x: i32, y: String },
}

struct Raw;

impl Serialize for Raw {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&[0, 127, 255])
    }
}

#[derive(Serialize)]
struct RawHolder {
    payload: Raw,
}

fn athlete_fixture() -> CanonicalAthlete {
    CanonicalAthlete::new(
        &SchoolId::mint("sch", &["canonical-json-fixture"]),
        "Synthetic Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "1001"),
    )
}

fn verdict_fixture() -> ReviewVerdictRecord {
    ReviewVerdictRecord {
        id: "athlete:sch:milesplit:1001:p1:0".to_string(),
        case_id: "athlete:sch:milesplit:1001:p1:0".to_string(),
        subject_id: "sch:milesplit:1001".to_string(),
        family: "Athlete identity unresolved".to_string(),
        kind: "merge".to_string(),
        field: "athlete".to_string(),
        value: "accept".to_string(),
        accepted: true,
        confidence: 3,
        rationale: "fixture rationale".to_string(),
        reviewer: "fixture-reviewer".to_string(),
        observed_at: "2026-09-29T00:00:00Z".to_string(),
        member_ids: Vec::new(),
    }
}

fn observations_fixture() -> Vec<SourceObservation> {
    vec![
        SourceObservation::School(SourceSchoolObservation::new(
            SourceNamespace::MilesplitSchool,
            "school-1",
            "row-1",
            "Fixture High",
            "2026-09-29",
        )),
        SourceObservation::Athlete(SourceAthleteObservation::new(
            SourceNamespace::MilesplitAthlete,
            "1001",
            "row-2",
            "Synthetic Runner",
            "2026-09-29",
        )),
    ]
}

fn case_fixture() -> ReviewCase {
    ReviewCase::pending(
        "Athlete identity unresolved",
        "sch:milesplit:1001",
        "Synthetic Runner",
        "two candidates share a name",
    )
}

#[test]
fn scalars_match_serde_json() {
    assert_matches(&());
    assert_matches(&true);
    assert_matches(&false);
    assert_matches(&i8::MIN);
    assert_matches(&i16::MIN);
    assert_matches(&i32::MIN);
    assert_matches(&i64::MIN);
    assert_matches(&i128::MIN);
    assert_matches(&u8::MAX);
    assert_matches(&u16::MAX);
    assert_matches(&u32::MAX);
    assert_matches(&u64::MAX);
    assert_matches(&u128::MAX);
    assert_matches(&0_i64);
    assert_matches(&1.0_f64);
    assert_matches(&1.5_f64);
    assert_matches(&-0.0_f64);
    assert_matches(&f64::MAX);
    assert_matches(&f32::MIN_POSITIVE);
    assert_matches(&'q');
    assert_matches(&'é');
    assert_matches(&None::<u8>);
    assert_matches(&Some(7_u8));
    assert_matches(&UnitStruct);
}

#[test]
fn non_finite_floats_match_serde_json_null() {
    assert_matches(&f64::NAN);
    assert_matches(&f64::INFINITY);
    assert_matches(&f64::NEG_INFINITY);
    assert_matches(&f32::NAN);
}

#[test]
fn strings_escape_like_serde_json() {
    assert_matches("plain");
    assert_matches("quote \" and backslash \\");
    assert_matches("\u{0}\u{1}\u{1f}\u{7f}");
    assert_matches("tab\t newline\n return\r backspace\u{8} form\u{c}");
    assert_matches("emoji 🏃 and accent é");
    assert_matches("");
}

#[test]
fn sequences_match_serde_json() {
    assert_matches(&Vec::<u8>::new());
    assert_matches(&vec![1_i32, 2, 3]);
    assert_matches(&(1_u8, "two", false));
    assert_matches(&TupleStruct(true, 'x'));
    assert_matches(&vec![vec![1_u8], vec![]]);
    assert_matches(&vec![Shape::Unit, Shape::Newtype(3), Shape::Tuple(4, 5)]);
    assert_matches(&RawHolder { payload: Raw });
}

#[test]
fn maps_and_structs_match_serde_json() {
    let mut map = BTreeMap::new();
    map.insert("alpha".to_string(), 1_i64);
    map.insert("beta".to_string(), -2);
    assert_matches(&map);
    assert_matches(&Inner {
        a: 9,
        b: Some("value".to_string()),
        c: vec![-1, 2],
    });
    assert_matches(&Newtype(u64::MAX));
    assert_matches(&Skipped {
        present: 1,
        absent: None,
    });
    assert_matches(&Flattened { id: 4, extra: map });
    assert_matches(&Shape::Struct {
        x: -1,
        y: "s".to_string(),
    });
}

#[test]
fn nested_domain_shaped_payload_matches_serde_json() {
    let payload = (
        Some(Shape::Tuple(1, 2)),
        vec![Inner {
            a: 1,
            b: None,
            c: vec![3],
        }],
        "tail",
    );
    assert_matches(&payload);
}

#[test]
fn production_payloads_match_serde_json() {
    let verdicts = vec![verdict_fixture()];
    let cases = vec![case_fixture()];
    assert_matches(&athlete_fixture());
    assert_matches(&verdicts);
    assert_matches(&cases);
    assert_matches(&(&verdicts, &cases));
    assert_matches(&observations_fixture());
}

#[test]
fn production_payload_digests_are_pinned() {
    let athlete = athlete_fixture();
    let verdicts = vec![verdict_fixture()];
    let cases = vec![case_fixture()];
    let observations = observations_fixture();
    let observed = [
        athlete_identity_digest(&athlete).expect("athlete digest"),
        identity_verdict_digest(&verdicts[0]).expect("verdict digest"),
        serialized_digest(&(&verdicts, &cases)).expect("checkpoint digest"),
        serialized_digest(&observations).expect("observation digest"),
    ];
    let reference = [
        reference_digest(&athlete),
        reference_digest(&verdicts[0]),
        reference_digest(&(&verdicts, &cases)),
        reference_digest(&observations),
    ];
    let expected = [
        "49c7a2d4124542842ca8b5acc540e3062924f6c05e0d9974e135c8c3889ecd87",
        "7bf1615a0747307e076817b152af5dc866bbcf177cc49fe3e4371bd9be3e91e7",
        "a4ac19817ef38ce5f66fda073cdff45ebec129c4091c75e2d8c76ab0274a3eed",
        "f579424dec5f0bd09e2524b5bbc0bd4016e3ba865b4c26cc8dda419ec1385702",
    ];
    assert_eq!(observed, reference);
    assert_eq!(observed, expected.map(str::to_string));
}

fn reference_digest<T: Serialize + ?Sized>(value: &T) -> String {
    use sha2::{Digest, Sha256};
    let bytes = serde_json::to_vec(value).expect("serde json write");
    format!("{:x}", Sha256::digest(&bytes))
}

#[test]
fn digest_is_sha256_of_canonical_bytes() {
    use sha2::{Digest, Sha256};
    let value = Inner {
        a: 5,
        b: Some("name".to_string()),
        c: vec![1, 2, 3],
    };
    let bytes = serialized_bytes(&value).expect("canonical json write");
    let digest = serialized_digest(&value).expect("canonical json digest");
    assert_eq!(digest, format!("{:x}", Sha256::digest(&bytes)));
    assert_eq!(digest.len(), 64);
    assert!(digest.bytes().all(|byte| byte.is_ascii_hexdigit()));
}

#[test]
fn unsupported_keys_are_refused() {
    let nested = BTreeMap::from([(vec![1_u8, 2], 3_i64)]);
    assert!(serialized_bytes(&nested).is_err());
    assert!(serde_json::to_vec(&nested).is_err());
}
