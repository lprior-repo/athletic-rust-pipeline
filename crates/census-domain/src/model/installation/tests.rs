use super::*;
use crate::model::{ContractRefV1, ProfileId, StoreIdentity};
use std::fmt::Debug;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn equal<T: Debug + PartialEq>(actual: T, expected: T) -> TestResult {
    if actual != expected {
        return Err(format!("actual={actual:?}; expected={expected:?}").into());
    }
    Ok(())
}

fn config() -> TestResult<InstallationConfigV1> {
    Ok(InstallationConfigV1::new(
        ProfileId::parse(&"a".repeat(64))?,
        ServiceResourceProfileV1::new(8, 48 * ServiceResourceProfileV1::GIB, 30)?,
        ContractRefV1::new(1, 1, &"b".repeat(64))?,
        ContractRefV1::new(1, 1, &"c".repeat(64))?,
        InstallationCapabilityV1::EnrollmentOnlyV1,
    ))
}

#[test]
fn installation_configuration_round_trips_and_order_does_not_change_identity() -> TestResult {
    let expected = config()?;
    let parsed: InstallationConfigV1 = serde_json::from_slice(&serde_json::to_vec(&expected)?)?;
    equal(parsed.id()?, expected.id()?)?;
    equal(parsed, expected.clone())?;
    let value = serde_json::to_value(&expected)?;
    let encoded = serde_json::to_vec_pretty(&value)?;
    let reordered: InstallationConfigV1 = serde_json::from_slice(&encoded)?;
    equal(reordered.id()?, expected.id()?)
}

#[test]
fn installation_configuration_rejects_unknown_fields_at_every_level() -> TestResult {
    for field in [
        None,
        Some("service_resources"),
        Some("source_registry"),
        Some("operating_defaults"),
    ] {
        let mut value = serde_json::to_value(config()?)?;
        let object = match field {
            None => &mut value,
            Some(field) => value.get_mut(field).ok_or("field")?,
        };
        object
            .as_object_mut()
            .ok_or("object")?
            .insert("extra".to_string(), serde_json::json!(true));
        if serde_json::from_value::<InstallationConfigV1>(value).is_ok() {
            return Err(format!("unknown field accepted at {field:?}").into());
        }
    }
    Ok(())
}

#[test]
fn installation_resources_preserve_the_existing_envelope_without_clamping() -> TestResult {
    let gib = ServiceResourceProfileV1::GIB;
    for concurrent in [1, 8, 1024] {
        for memory in [gib, 48 * gib, ServiceResourceProfileV1::MAX_MEMORY_BYTES] {
            for drain in [0, 30, u64::MAX] {
                let resources = ServiceResourceProfileV1::new(concurrent, memory, drain)?;
                equal(resources.max_concurrent(), concurrent)?;
                equal(resources.memory_budget_bytes(), memory)?;
                equal(resources.drain_timeout_seconds(), drain)?;
            }
        }
    }
    for concurrent in [0, 1025, usize::MAX] {
        if !matches!(
            ServiceResourceProfileV1::new(concurrent, gib, 30),
            Err(InstallationDomainError::InvalidServiceResource {
                field: "max_concurrent"
            })
        ) {
            return Err("concurrency boundary accepted".into());
        }
    }
    for memory in [
        0,
        gib.checked_sub(1).ok_or("gib")?,
        gib.checked_add(1).ok_or("gib")?,
        ServiceResourceProfileV1::MAX_MEMORY_BYTES
            .checked_add(1)
            .ok_or("ceiling")?,
        u64::MAX,
    ] {
        if !matches!(
            ServiceResourceProfileV1::new(8, memory, 30),
            Err(InstallationDomainError::InvalidServiceResource {
                field: "memory_budget_bytes"
            })
        ) {
            return Err("memory boundary accepted".into());
        }
    }
    Ok(())
}

#[test]
fn installation_identity_changes_for_each_semantic_field() -> TestResult {
    let original = config()?;
    let id = original.id()?;
    for (field, value) in [
        ("initial_profile_id", serde_json::json!("d".repeat(64))),
        (
            "service_resources",
            serde_json::json!({"max_concurrent":9,"memory_budget_bytes":48 * ServiceResourceProfileV1::GIB,"drain_timeout_seconds":30}),
        ),
        (
            "service_resources",
            serde_json::json!({"max_concurrent":8,"memory_budget_bytes":49 * ServiceResourceProfileV1::GIB,"drain_timeout_seconds":30}),
        ),
        (
            "service_resources",
            serde_json::json!({"max_concurrent":8,"memory_budget_bytes":48 * ServiceResourceProfileV1::GIB,"drain_timeout_seconds":31}),
        ),
        (
            "source_registry",
            serde_json::json!({"version":1,"revision":2,"digest":"b".repeat(64)}),
        ),
        (
            "source_registry",
            serde_json::json!({"version":1,"revision":1,"digest":"d".repeat(64)}),
        ),
        (
            "operating_defaults",
            serde_json::json!({"version":1,"revision":2,"digest":"c".repeat(64)}),
        ),
        (
            "operating_defaults",
            serde_json::json!({"version":1,"revision":1,"digest":"d".repeat(64)}),
        ),
    ] {
        let mut value_original = serde_json::to_value(&original)?;
        value_original
            .as_object_mut()
            .ok_or("config")?
            .insert(field.to_string(), value);
        let changed: InstallationConfigV1 = serde_json::from_value(value_original)?;
        if changed.id()? == id {
            return Err(format!("{field} did not alter identity").into());
        }
    }
    Ok(())
}

#[test]
fn installation_unknown_version_capability_and_digest_refuse() -> TestResult {
    let expected = config()?;
    for version in [0, 2, u32::MAX] {
        let mut value = serde_json::to_value(&expected)?;
        value
            .as_object_mut()
            .ok_or("config")?
            .insert("version".to_string(), serde_json::json!(version));
        if serde_json::from_value::<InstallationConfigV1>(value).is_ok() {
            return Err("version accepted".into());
        }
    }
    for capability in ["", "reusable_production", "enrollment_only_v2"] {
        let mut value = serde_json::to_value(&expected)?;
        value
            .as_object_mut()
            .ok_or("config")?
            .insert("capability".to_string(), serde_json::json!(capability));
        if serde_json::from_value::<InstallationConfigV1>(value).is_ok() {
            return Err("capability accepted".into());
        }
    }
    for digest in ["A".repeat(64), "a".repeat(63), "g".repeat(64)] {
        if InstallationConfigId::parse(&digest).is_ok() {
            return Err("noncanonical digest accepted".into());
        }
    }
    if ContractRefV1::new(1, 0, &"a".repeat(64)).is_ok() {
        return Err("zero reference revision accepted".into());
    }
    let identity = StoreIdentity::parse(&"A".repeat(64))?;
    let enrollment = InstallationEnrollmentV1::new(identity.clone(), expected.id()?);
    let reopened: InstallationEnrollmentV1 =
        serde_json::from_slice(&serde_json::to_vec(&enrollment)?)?;
    equal(reopened.store_identity(), &identity)
}
