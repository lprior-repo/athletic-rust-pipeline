use super::*;
use serde::Deserialize;

pub(super) const CONTRACT: &str =
    include_str!("../../../../tests/fixtures/region_membership/synthetic-contract.json");
const EXPECTED: &str =
    include_str!("../../../../tests/fixtures/region_membership/expected-school-resolutions.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedSchool {
    school: SchoolId,
    verdict: String,
    applied_statements: Vec<String>,
    relevant_locators: Vec<String>,
}

#[test]
fn independent_synthetic_fixture_preserves_exact_membership_and_provenance() -> TestResult {
    admit_region_payload_bytes_v1(CONTRACT.as_bytes())?;
    let payload: RegionMembershipContractV1 = serde_json::from_str(CONTRACT)?;
    let expected: Vec<ExpectedSchool> = serde_json::from_str(EXPECTED)?;
    check!(eq; payload.input().assertions.len(), 6);
    check!(eq; expected.len(), 5);
    for expected in expected {
        let school = RegionSchoolV1::new(expected.school, UsJurisdiction::Michigan)?;
        let query = query(RegionMembershipTargetV1::School(school), "2026-10-01")?;
        let result = resolve_region_membership_v1(&payload, &query);
        check!(eq; serde_json::to_value(result.verdict())?, serde_json::Value::String(expected.verdict));
        let mut applied = Vec::new();
        let mut locators = Vec::new();
        for trace in result.trace() {
            if trace.disposition != RegionAssertionDispositionV1::OtherSubject {
                locators.push(trace.assertion.input().provenance.locator().to_string());
            }
            if trace.disposition == RegionAssertionDispositionV1::Applied {
                applied.push(serde_json::to_value(trace.assertion.input().statement)?);
            }
        }
        let expected_applied: Vec<serde_json::Value> = expected
            .applied_statements
            .into_iter()
            .map(serde_json::Value::String)
            .collect();
        check!(eq; applied, expected_applied);
        locators.sort_unstable();
        check!(eq; locators, expected.relevant_locators);
    }
    Ok(())
}

#[test]
fn independent_fixture_co_op_does_not_override_explicit_constituent_school_exclusion() -> TestResult
{
    let payload: RegionMembershipContractV1 = serde_json::from_str(CONTRACT)?;
    let b: SchoolId = serde_json::from_str("\"sch_0000000000000002\"")?;
    let b = RegionSchoolV1::new(b, UsJurisdiction::Michigan)?;
    let program: TeamId = serde_json::from_str("\"team_0000000000000001\"")?;
    let program_query = query(
        RegionMembershipTargetV1::Program {
            program,
            affiliated_school: b.clone(),
        },
        "2026-10-01",
    )?;
    let school_query = query(RegionMembershipTargetV1::School(b), "2026-10-01")?;
    check!(eq; resolve_region_membership_v1(&payload, &program_query).verdict(), RegionMembershipVerdictV1::Member);
    check!(eq; resolve_region_membership_v1(&payload, &school_query).verdict(), RegionMembershipVerdictV1::ExplicitNonMember);
    Ok(())
}

#[test]
fn canonical_set_order_deduplicates_only_byte_exact_semantic_provenance() -> TestResult {
    let original: RegionMembershipContractV1 = serde_json::from_str(CONTRACT)?;
    let mut reordered = original.input().clone();
    reordered.assertions.reverse();
    reordered
        .assertions
        .extend(original.input().assertions.clone());
    reordered
        .jurisdictions
        .extend(original.input().jurisdictions.clone());
    let equivalent = RegionMembershipContractV1::new(reordered)?;
    check!(eq; crate::model::serialized_bytes(&original)?, crate::model::serialized_bytes(&equivalent)?);
    check!(eq; original.reference()?, equivalent.reference()?);
    let first = original
        .input()
        .assertions
        .first()
        .ok_or_else(|| std::io::Error::other("missing fixture assertion"))?;
    let mut another_capture = first.input().clone();
    let mut provenance = another_capture.provenance.input().clone();
    provenance.capture_sha256 = "d".repeat(64);
    another_capture.provenance = RegionEvidenceV1::new(provenance)?;
    let mut expanded = original.input().clone();
    expanded
        .assertions
        .push(RegionAssertionV1::new(another_capture));
    let changed = RegionMembershipContractV1::new(expanded)?;
    check!(eq; changed.input().assertions.len(), 7);
    check!(ne; original.reference()?, changed.reference()?);
    let roundtrip: RegionMembershipContractV1 =
        serde_json::from_slice(&crate::model::serialized_bytes(&changed)?)?;
    check!(eq; changed, roundtrip);
    Ok(())
}
