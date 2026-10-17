use super::*;
use crate::check;
use crate::evidence_subject_attachment::*;
use crate::model::*;
use crate::public_identity_history::PublicAthleteId;

mod budget;
mod fixture;
mod invalid;
mod inventory;
mod replay;
use fixture::*;
type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn syntactic_command_retains_original_grant_without_school_admission() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    let checked = CheckedResultSchoolCommandV1::check(command("controlled-first")?, &mut grant)?;
    check!(eq; checked.command(&mut grant)?.version, 1);
    let mut other = AttachmentReplayGrantV1::new();
    check!(checked.command(&mut other).is_err());
    check!(other.ensure_available().is_err());
    check!(grant.ensure_available().is_ok());
    Ok(())
}

#[test]
fn exact_statement_boundaries_and_capacity_fail_closed() -> TestResult {
    for (offset, length, accepted) in [
        (0, 100, true),
        (99, 1, true),
        (100, 1, false),
        (0, 0, false),
        (u64::MAX, 1, false),
    ] {
        let mut value = command("controlled-bounds")?;
        value.source.statement.statement_offset = offset;
        value.source.statement.statement_length = length;
        let label = "x".repeat(usize::try_from(length.max(1))?);
        value.source.statement.statement_text = text(&label)?;
        value.source.statement.statement_digest = raw_digest(&label)?;
        let mut grant = AttachmentReplayGrantV1::new();
        let result = CheckedResultSchoolCommandV1::check(value, &mut grant);
        check!(eq; result.is_ok(), accepted);
        check!(eq; grant.ensure_available().is_ok(), accepted);
    }
    Ok(())
}

#[test]
fn exact_object_limit_and_duplicate_inventory_are_checked() -> TestResult {
    for count in [MAX_RESULT_SCHOOL_OBJECTS, MAX_RESULT_SCHOOL_OBJECTS + 1] {
        let mut value = command("controlled-object-limit")?;
        value.source.evidence = AttachmentListV1::new(
            (0..count)
                .map(|index| object(&format!("controlled-{index}")))
                .collect::<TestResult<Vec<_>>>()?,
        )?;
        let mut grant = AttachmentReplayGrantV1::new();
        check!(eq; CheckedResultSchoolCommandV1::check(value, &mut grant).is_ok(), count == MAX_RESULT_SCHOOL_OBJECTS);
    }
    let mut value = command("controlled-duplicate")?;
    value.source.evidence = AttachmentListV1::new(vec![object("same")?, object("same")?])?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(CheckedResultSchoolCommandV1::check(value, &mut grant).is_err());
    check!(grant.ensure_available().is_err());
    Ok(())
}

#[test]
fn canonical_decoding_and_digest_refusal_poison_grant() -> TestResult {
    let value = record(command("controlled-canonical")?)?;
    let canonical = crate::model::serialized_bytes(&value)?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(eq; decode_result_school_record_v1(&canonical, &mut grant)?, value);
    let mut noncanonical = canonical;
    noncanonical.push(b'\n');
    let mut grant = AttachmentReplayGrantV1::new();
    check!(decode_result_school_record_v1(&noncanonical, &mut grant).is_err());
    check!(grant.ensure_available().is_err());
    let mut grant = AttachmentReplayGrantV1::new();
    check!(
        result_school_digest_v1(&"x".repeat(MAX_RESULT_SCHOOL_RECORD_BYTES), &mut grant).is_err()
    );
    check!(grant.ensure_available().is_err());
    Ok(())
}

#[test]
fn predecessor_capability_wire_preserves_seven_versus_eight() -> TestResult {
    let seven = ResultSchoolPriorCapabilityV1::IdentityOnly {
        qualification: digest("qualification")?,
    };
    let eight = ResultSchoolPriorCapabilityV1::IdentityAndRosterSource {
        qualification: digest("qualification")?,
        roster_enrollment: digest("enrollment")?,
    };
    check!(eq; seven.lifecycle(), 7);
    check!(eq; eight.lifecycle(), 8);
    check!(eq; seven.qualification(), eight.qualification());
    for capability in [seven, eight] {
        let bytes = crate::model::serialized_bytes(&capability)?;
        let replayed: ResultSchoolPriorCapabilityV1 = serde_json::from_slice(&bytes)?;
        check!(eq; replayed, capability);
        check!(eq; crate::model::serialized_bytes(&replayed)?, bytes);
    }
    check!(serde_json::from_str::<ResultSchoolPriorCapabilityV1>(
        r#"{"IdentityOnly":{"qualification":"bad"}}"#
    )
    .is_err());
    Ok(())
}
