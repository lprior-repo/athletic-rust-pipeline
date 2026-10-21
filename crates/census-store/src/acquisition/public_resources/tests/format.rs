use super::super::{codec, format, PublicResourceRootKindV1, DECLARATION, ENTRY_PREFIX};
use super::fixtures::*;

#[test]
fn declaration_and_frontier_namespaces_require_exact_derived_keys() -> TestResult {
    let f = fixture("2026-10-21")?;
    let held = entry(&f.record, PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    format::validate_declaration_key(DECLARATION.as_bytes())?;
    format::validate_entry_key(format!("{ENTRY_PREFIX}{}", held.id()?).as_bytes(), &held)?;
    for suffix in ["/extra", "\0", " "] {
        refusal(
            format::validate_declaration_key(format!("{DECLARATION}{suffix}").as_bytes()),
            "public resource declaration namespace",
        )?;
    }
    let other = entry(&f.record, PublicResourceRootKindV1::Robots)?;
    refusal(
        format::validate_entry_key(format!("{ENTRY_PREFIX}{}", other.id()?).as_bytes(), &held),
        "public resource frontier namespace",
    )
}

#[test]
fn frontier_receipt_binds_entry_digest_operation_clock_and_zero_credit() -> TestResult {
    let f = fixture("2026-10-21")?;
    let held = entry(&f.record, PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    let receipt = entry_receipt(&f.record, &held)?;
    format::validate_entry_receipt(&f.record, &held, &receipt)?;
    let mut earlier = receipt.clone();
    earlier.at = "2026-10-21T07:59:59Z".to_string();
    let mut digest = receipt.clone();
    digest.digest = "b".repeat(64);
    let mut operation = receipt.clone();
    operation.operation.push_str(":other");
    let mut appended = receipt.clone();
    appended.appended = 1;
    let mut credited = receipt.clone();
    credited.credited = true;
    for forged in [earlier, digest, operation, appended, credited] {
        refusal(
            format::validate_entry_receipt(&f.record, &held, &forged),
            "public resource frontier receipt",
        )?;
    }
    let mut malformed = receipt;
    malformed.at = "not-a-clock".to_string();
    refusal(
        format::validate_entry_receipt(&f.record, &held, &malformed),
        "public resource frontier receipt time",
    )
}

#[test]
fn protected_prefixes_refuse_generic_writes_including_unknown_aliases() -> TestResult {
    for prefix in ["public_resource_capture", "public_resource_frontier"] {
        for suffix in ["", "/v1", "\0", "_v9:write"] {
            let key = format!("{prefix}{suffix}");
            equal(format::reserved_family(key.as_bytes()), true)?;
            equal(format::is_reserved_operation(key.as_bytes()), true)?;
            refusal(
                format::refuse_generic_phase(&key),
                "public resource owned phase required",
            )?;
            refusal(
                format::refuse_generic_operation(&key),
                "public resource owned operation required",
            )?;
        }
    }
    equal(
        format::reserved_family(b"ordinary-public-resource-note"),
        false,
    )?;
    format::refuse_generic_phase("ordinary-public-resource-note")?;
    format::refuse_generic_operation("ordinary-public-resource-note")?;
    Ok(())
}

#[test]
fn complete_declaration_validation_checks_lineage_before_held_receipt() -> TestResult {
    let f = fixture("2026-10-21")?;
    let receipt = f.record.receipt()?;
    format::validate_declaration(&f.record, &f.capture, &f.manifest, &f.profile, &receipt)?;
    let mut foreign_manifest = f.manifest.clone();
    foreign_manifest.store_identity = "b".repeat(64);
    let mut rehashed = f.record.clone();
    rehashed.binding.run_manifest_digest = codec::object_digest(&foreign_manifest)?;
    let self_consistent_receipt = rehashed.receipt()?;
    refusal(
        format::validate_declaration(
            &rehashed,
            &f.capture,
            &foreign_manifest,
            &f.profile,
            &self_consistent_receipt,
        ),
        "public resource declaration lineage",
    )
}
