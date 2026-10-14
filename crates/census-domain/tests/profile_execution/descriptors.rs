use super::super::*;
use super::common::{equal, TestResult};
use sha2::{Digest, Sha256};

const ORACLES: [(ContractRoleV1, &[u8], &str); 9] = [
    (
        ContractRoleV1::Cohort,
        include_bytes!("fixtures/cohort.canonical.json"),
        "596cb3d3d6722b2ef8a089da773d2d54c982a086b03c594118b07f46fa442f70",
    ),
    (
        ContractRoleV1::Event,
        include_bytes!("fixtures/event.canonical.json"),
        "95746745f36a7bc3354695d8ae8998ebe3710fe941c7ca39084c321af62d7b42",
    ),
    (
        ContractRoleV1::Identity,
        include_bytes!("fixtures/identity.canonical.json"),
        "d91af128fe3089b27c7a7964517bb18b63877f9d466e76db2f64aa2932ccfca0",
    ),
    (
        ContractRoleV1::Affiliation,
        include_bytes!("fixtures/affiliation.canonical.json"),
        "adbdb04c9b7c5edda39800a33d65fb4851c8a8d0925e4a14e4749a1fd66f2416",
    ),
    (
        ContractRoleV1::Contact,
        include_bytes!("fixtures/contact.canonical.json"),
        "82c645d7c29a8eed71dcfe37918bb4fe903ad3f922b86e5bb1d077a35e847097",
    ),
    (
        ContractRoleV1::Participation,
        include_bytes!("fixtures/participation.canonical.json"),
        "f276cbfee684b4af83a897f9d6f9de42097c69f5bcc105797fe8c755676bb312",
    ),
    (
        ContractRoleV1::Publication,
        include_bytes!("fixtures/publication.canonical.json"),
        "ff8dd79dbb18f925e0b6d830500d594ac507347ba32da2e6888b4d2c4121c84d",
    ),
    (
        ContractRoleV1::SourcePlan,
        include_bytes!("fixtures/source_plan.canonical.json"),
        "29768a1f0b279d8dde04f574559eadbc8195e9b6e8d2e8f9ba83722f3be659e2",
    ),
    (
        ContractRoleV1::Completion,
        include_bytes!("fixtures/completion.canonical.json"),
        "30df2b60e32d6d1624b2bca01349a7f27ff90d2e1d5a23d0ae2e09391a2dd017",
    ),
];

#[test]
fn role_order_and_every_descriptor_have_independent_fixed_byte_and_digest_oracles() -> TestResult {
    for ((role, bytes, hash), expected_role) in ORACLES.into_iter().zip(ContractRoleV1::ALL) {
        equal(role, expected_role)?;
        let descriptor = profile_contract_descriptor_v1(role)?;
        equal(descriptor.role(), role)?;
        equal(descriptor.version(), 1)?;
        equal(descriptor.revision(), 1)?;
        equal(serialized_bytes(&descriptor)?.as_slice(), bytes)?;
        equal(serde_json::to_vec(&descriptor)?.as_slice(), bytes)?;
        equal(format!("{:x}", Sha256::digest(bytes)).as_str(), hash)?;
        let reference = profile_contract_ref_v1(role)?;
        equal(reference.digest(), hash)?;
        equal(reference.revision().get(), 1)?;
        let expected = format!("{{\"version\":1,\"revision\":1,\"digest\":\"{hash}\"}}");
        equal(
            serialized_bytes(&reference)?.as_slice(),
            expected.as_bytes(),
        )?;
    }
    Ok(())
}

#[test]
fn each_role_has_a_distinct_exact_reference() -> TestResult {
    for (role, _, hash) in ORACLES {
        for (other, _, other_hash) in ORACLES {
            if role != other && hash == other_hash {
                return Err(
                    format!("distinct {role:?}/{other:?} descriptor identities collided").into(),
                );
            }
        }
    }
    Ok(())
}

#[test]
fn event_descriptor_binds_the_exact_corrected_catalog_policy_bytes() -> TestResult {
    let catalog = RecruitingCatalogV1::new();
    let expected = include_bytes!("fixtures/catalog-policy.canonical.json");
    let mut actual = Vec::new();
    catalog.write_policy(&mut actual)?;
    equal(actual.as_slice(), expected.as_slice())?;
    equal(
        catalog.digest()?.as_str(),
        "6f301c435cfe971548c4ede977c780b66b5456591bf70fe08711398b5bc1c1d5",
    )?;
    let descriptor = serde_json::to_value(profile_contract_descriptor_v1(ContractRoleV1::Event)?)?;
    let nested = descriptor
        .get("policy")
        .and_then(|policy| policy.get("components"))
        .and_then(|components| components.get("catalog"))
        .ok_or("catalog component")?;
    let reference: ContractRefV1 = serde_json::from_value(nested.clone())?;
    equal(reference.digest(), catalog.digest()?.as_str())
}

#[test]
fn descriptors_bind_evidence_roles_and_disclose_unimplemented_execution() -> TestResult {
    for (role, _, _) in ORACLES {
        let descriptor = serde_json::to_value(profile_contract_descriptor_v1(role)?)?;
        equal(
            descriptor
                .get("binding")
                .and_then(serde_json::Value::as_str),
            Some("configuration_only"),
        )?;
        equal(
            descriptor
                .get("runtime_execution")
                .and_then(serde_json::Value::as_str),
            Some("not_certified"),
        )?;
        let policy = descriptor.get("policy").ok_or("policy")?;
        let rule = policy
            .get("evidence_rule")
            .and_then(serde_json::Value::as_str)
            .ok_or("evidence rule")?;
        let support = policy
            .get("component_support")
            .and_then(serde_json::Value::as_str)
            .ok_or("component support")?;
        match role {
            ContractRoleV1::Identity => equal(
                rule,
                "captured_primary_or_independent_attested_person_keys_hard_contradictions_withheld",
            )?,
            ContractRoleV1::Affiliation => {
                equal(support, "dated_current_affiliation_unimplemented")?
            }
            ContractRoleV1::SourcePlan => equal(
                support,
                "profile_bound_qualified_source_graph_unimplemented",
            )?,
            ContractRoleV1::Completion => {
                equal(support, "profile_bound_completion_fence_unimplemented")?
            }
            ContractRoleV1::Cohort
            | ContractRoleV1::Event
            | ContractRoleV1::Contact
            | ContractRoleV1::Participation
            | ContractRoleV1::Publication => {}
        }
    }
    Ok(())
}
