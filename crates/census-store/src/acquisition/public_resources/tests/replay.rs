use super::super::*;
use super::fixtures::*;
use census_domain::model::AcquisitionDigest;

#[test]
fn exact_replay_returns_original_receipt_and_admission_timestamp() -> TestResult {
    let f = fixture("2026-10-21")?;
    let receipt = f.record.receipt()?;
    let held_bytes = codec::encode(&f.record)?;
    let later =
        PublicResourceDeclarationV1::new(f.record.binding.clone(), at("2026-10-22T09:30:00Z")?)?;
    equal(
        later.binding.plan_digest()?,
        f.record.binding.plan_digest()?,
    )?;
    equal(later.digest()? == f.record.digest()?, false)?;
    let returned = f.record.replay(&later.binding, &receipt)?;
    equal(std::ptr::eq(returned, &receipt), true)?;
    equal(returned, &receipt)?;
    equal(returned.at.as_str(), "2026-10-21T08:00:00+00:00")?;
    equal(codec::encode(&f.record)?, held_bytes)?;
    equal(f.record.admitted_at, at("2026-10-21T08:00:00Z")?)
}

#[test]
fn binding_changes_refuse_held_receipt_without_replacing_original() -> TestResult {
    let f = fixture("2026-10-21")?;
    let receipt = f.record.receipt()?;
    let before = codec::encode(&f.record)?;
    let mut national = f.record.binding.clone();
    national.national_request_digest = AcquisitionDigest::parse(&"b".repeat(64))?;
    let mut owner = f.record.binding.clone();
    owner
        .owners
        .first_mut()
        .ok_or("fixture owner")?
        .jurisdiction_request_digest = AcquisitionDigest::parse(&"c".repeat(64))?;
    let as_of = fixture("2026-10-20")?;
    for changed in [national, owner, as_of.record.binding] {
        refusal(
            f.record.replay(&changed, &receipt),
            "public resource declaration binding conflict",
        )?;
        equal(codec::encode(&f.record)?, before.clone())?;
        equal(f.record.receipt()?, receipt.clone())?;
    }
    Ok(())
}

#[test]
fn receipt_timestamp_digest_operation_and_effect_fields_are_exact() -> TestResult {
    let f = fixture("2026-10-21")?;
    let held = f.record.receipt()?;
    let mut at = held.clone();
    at.at = "2026-10-22T08:00:00+00:00".to_string();
    let mut digest = held.clone();
    digest.digest = "b".repeat(64);
    let mut operation = held.clone();
    operation.operation.push_str(":other");
    let mut appended = held.clone();
    appended.appended = 1;
    let mut credited = held.clone();
    credited.credited = true;
    for forged in [at, digest, operation, appended, credited] {
        refusal(
            f.record.replay(&f.record.binding, &forged),
            "public resource declaration receipt",
        )?;
    }
    equal(f.record.receipt()?, held)
}
