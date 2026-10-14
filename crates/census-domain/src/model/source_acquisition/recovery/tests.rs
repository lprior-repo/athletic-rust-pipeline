use super::super::{AcquisitionWorkId, AttemptOrdinal, CaptureQualificationV2};
use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn target() -> Result<PreparedRecoveryTarget, Box<dyn std::error::Error>> {
    let identity = StoreIdentity::parse(&"Ab".repeat(32))?;
    Ok(PreparedRecoveryTarget::new(
        identity.clone(),
        AcquisitionSlotId::parse(&"b".repeat(64))?,
        AcquisitionAttemptId::new(
            identity,
            AcquisitionWorkId::parse(&"c".repeat(64))?,
            AttemptOrdinal::new(1)?,
        ),
        AcquisitionDigest::parse(&"d".repeat(64))?,
    )?)
}

#[test]
fn v2_qualification_has_exact_six_field_wire_and_no_v1_protocol_fallback() -> TestResult {
    let identity = StoreIdentity::parse(&"Ab".repeat(32))?;
    let record = CaptureQualificationV2::new(identity.clone());
    let expected = format!(
        r#"{{"version":2,"lifecycle":"fresh_qualification","store_identity":"{}","protocol":"source_acquisition_attempt_v2","consumer":"milesplit_structured_whole_meet_v1","cache_envelope_version":2}}"#,
        identity.as_str()
    );
    check!(eq; serde_json::to_string(&record)?, expected.clone());
    check!(eq; serde_json::from_str::<CaptureQualificationV2>(&expected)?.store_identity(), &identity);
    for (label, raw) in [
        ("v1 version", expected.replace("\"version\":2", "\"version\":1")),
        ("v3 version", expected.replace("\"version\":2", "\"version\":3")),
        ("v1 protocol", expected.replace("attempt_v2", "attempt_v1")),
        ("duplicate version", expected.replacen("{", "{\"version\":2,", 1)),
        ("unknown field", expected.replacen("{", "{\"unknown\":0,", 1)),
    ] {
        data_error::<CaptureQualificationV2>(label, &raw)?;
    }
    empty_error::<CaptureQualificationV2>()?;
    Ok(())
}

#[test]
fn recovery_target_and_decision_strictly_bind_repeated_store_identity() -> TestResult {
    let target = target()?;
    let assessment =
        MissingPreparedAssessmentV1::new(MissingLocation::BodyLeaf, MissingMetadataV1::Exact)?;
    let decision = MissingPreparedDecisionV1::new(target.clone(), assessment);
    let target_raw = serde_json::to_string(&target)?;
    let decision_raw = serde_json::to_string(&decision)?;
    check!(eq; serde_json::from_str::<PreparedRecoveryTarget>(&target_raw)?, target.clone());
    check!(eq; serde_json::from_str::<MissingPreparedDecisionV1>(&decision_raw)?, decision.clone());
    check!(decision_raw.len() <= 4096);
    for (label, raw) in [
        ("foreign Store", target_raw.replacen(target.store_identity().as_str(), &"e".repeat(64), 1)),
        ("duplicate identity", target_raw.replacen("{", &format!("{{\"store_identity\":\"{}\",", target.store_identity().as_str()), 1)),
        ("unknown field", target_raw.replacen("{", "{\"extra\":0,", 1)),
        ("zero ordinal", target_raw.replace("\"ordinal\":1", "\"ordinal\":0")),
    ] {
        data_error::<PreparedRecoveryTarget>(label, &raw)?;
    }
    empty_error::<PreparedRecoveryTarget>()?;
    for (label, raw) in [
        ("foreign Store", decision_raw.replacen(target.store_identity().as_str(), &"e".repeat(64), 1)),
        ("wrong version", decision_raw.replacen("\"version\":1", "\"version\":2", 1)),
        ("duplicate version", decision_raw.replacen("{", "{\"version\":1,", 1)),
        ("unknown field", decision_raw.replacen("{", "{\"extra\":0,", 1)),
        ("unknown reason", decision_raw.replace("close_without_supplied_exact_body", "ignored")),
    ] {
        data_error::<MissingPreparedDecisionV1>(label, &raw)?;
    }
    empty_error::<MissingPreparedDecisionV1>()?;
    Ok(())
}

#[test]
fn missing_assessment_has_exact_wire_digest_and_rejects_wrong_role_shared_parent() -> TestResult {
    let target = target()?;
    let assessment =
        MissingPreparedAssessmentV1::new(MissingLocation::BodyLeaf, MissingMetadataV1::Exact)?;
    let expected = r#"{"version":1,"body":{"kind":"missing","location":"body_leaf"},"metadata":{"kind":"exact"}}"#;
    check!(eq; serde_json::to_string(&assessment)?, expected);
    check!(eq; assessment.digest(&target)?.as_str(), crate::model::serialized_digest(&("prepared-missing-assessment-v1", target.store_identity(), target.slot_id(), target.attempt(), target.prepared_digest(), &assessment))?);
    let decision = MissingPreparedDecisionV1::new(target, assessment);
    check!(eq; decision.digest()?.as_str(), crate::model::serialized_digest(&("prepared-missing-decision-v1", &decision))?);
    for (label, raw) in [
        ("wrong body role", expected.replace("body_leaf", "metadata_leaf")),
        ("removed HTTP role", expected.replace("body_leaf", "http_directory")),
        ("shared parent", expected.replace("body_leaf", "archive_directory")),
        ("wrong metadata role", expected.replace("\"exact\"", "\"missing\",\"location\":\"bodies_directory\"")),
        ("wrong version", expected.replace("\"version\":1", "\"version\":2")),
        ("duplicate version", expected.replacen("{", "{\"version\":1,", 1)),
        ("unknown field", expected.replacen("{", "{\"extra\":0,", 1)),
    ] {
        data_error::<MissingPreparedAssessmentV1>(label, &raw)?;
    }
    empty_error::<MissingPreparedAssessmentV1>()?;
    check!(matches!(
        MissingPreparedAssessmentV1::new(
            MissingLocation::CapturesDirectory,
            MissingMetadataV1::Exact
        ),
        Err(CaptureDomainError::Invalid {
            field: "missing body location"
        })
    ));
    check!(matches!(
        MissingPreparedAssessmentV1::new(
            MissingLocation::BodyLeaf,
            MissingMetadataV1::Missing {
                location: MissingLocation::BodiesDirectory
            }
        ),
        Err(CaptureDomainError::Invalid {
            field: "missing metadata location"
        })
    ));
    check!(matches!(
        MissingPreparedAssessmentV1::new(
            MissingLocation::ArchiveDirectory,
            MissingMetadataV1::Exact
        ),
        Err(CaptureDomainError::Invalid {
            field: "shared missing archive parent"
        })
    ));
    Ok(())
}

fn data_error<T: serde::de::DeserializeOwned>(
    label: &str,
    raw: &str,
) -> TestResult {
    let error = serde_json::from_str::<T>(raw)
        .err()
        .ok_or("accepted hostile wire")?;
    check!(eq; (label, error.classify()), (label, serde_json::error::Category::Data));
    Ok(())
}
fn empty_error<T: serde::de::DeserializeOwned>() -> TestResult {
    let error = serde_json::from_str::<T>("")
        .err()
        .ok_or("accepted empty wire")?;
    check!(eq; error.classify(), serde_json::error::Category::Eof);
    Ok(())
}

#[test]
fn every_mandatory_recovery_field_has_exact_missing_field_error() -> TestResult {
    let target = target()?;
    let assessment =
        MissingPreparedAssessmentV1::new(MissingLocation::BodyLeaf, MissingMetadataV1::Exact)?;
    let decision = MissingPreparedDecisionV1::new(target.clone(), assessment.clone());
    required_fields::<PreparedRecoveryTarget>(
        &target,
        &["store_identity", "slot_id", "attempt", "prepared_digest"],
    )?;
    required_fields::<MissingPreparedAssessmentV1>(&assessment, &["version", "body", "metadata"])?;
    required_fields::<MissingPreparedDecisionV1>(
        &decision,
        &[
            "version",
            "store_identity",
            "slot_id",
            "attempt",
            "prepared_digest",
            "assessment",
            "reason",
        ],
    )?;
    required_fields::<CaptureQualificationV2>(
        &CaptureQualificationV2::new(target.store_identity().clone()),
        &[
            "version",
            "lifecycle",
            "store_identity",
            "protocol",
            "consumer",
            "cache_envelope_version",
        ],
    )?;
    Ok(())
}
fn required_fields<T: serde::de::DeserializeOwned>(
    value: &impl serde::Serialize,
    fields: &[&str],
) -> TestResult {
    for field in fields {
        let mut raw = serde_json::to_value(value)?;
        raw.as_object_mut()
            .ok_or("object")?
            .remove(*field)
            .ok_or("required field")?;
        let text = serde_json::to_string(&raw)?;
        let error = serde_json::from_str::<T>(&text)
            .err()
            .ok_or("accepted missing field")?;
        check!(error.is_data());
    }
    Ok(())
}

#[test]
fn recovery_domain_foreign_identity_and_role_refusals_are_exact() -> TestResult {
    let target = target()?;
    let foreign = StoreIdentity::parse(&"e".repeat(64))?;
    check!(matches!(
        PreparedRecoveryTarget::new(
            foreign,
            target.slot_id().clone(),
            target.attempt().clone(),
            target.prepared_digest().clone()
        ),
        Err(CaptureDomainError::Invalid {
            field: "recovery Store identity"
        })
    ));
    for location in [
        MissingLocation::CapturesDirectory,
        MissingLocation::ContentCaptureDirectory,
        MissingLocation::MetadataLeaf,
    ] {
        check!(matches!(
            MissingPreparedAssessmentV1::new(location, MissingMetadataV1::Exact),
            Err(CaptureDomainError::Invalid {
                field: "missing body location"
            })
        ));
    }
    for location in [MissingLocation::BodiesDirectory, MissingLocation::BodyLeaf] {
        check!(matches!(
            MissingPreparedAssessmentV1::new(
                MissingLocation::BodyLeaf,
                MissingMetadataV1::Missing { location }
            ),
            Err(CaptureDomainError::Invalid {
                field: "missing metadata location"
            })
        ));
    }
    check!(matches!(
        MissingPreparedAssessmentV1::new(
            MissingLocation::BodyLeaf,
            MissingMetadataV1::Missing {
                location: MissingLocation::ArchiveDirectory
            }
        ),
        Err(CaptureDomainError::Invalid {
            field: "shared missing archive parent"
        })
    ));
    Ok(())
}

#[test]
fn fallible_constrained_copies_preserve_equality_wire_and_digest() -> TestResult {
    let target = target()?;
    let copied = target.try_clone()?;
    check!(eq; copied, target);
    check!(eq; serde_json::to_vec(&copied)?, serde_json::to_vec(&target)?);
    check!(eq; crate::model::serialized_digest(&copied)?, crate::model::serialized_digest(&target)?);
    check!(eq; copied.store_identity().as_str(), "Ab".repeat(32));
    Ok(())
}
