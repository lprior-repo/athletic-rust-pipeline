use super::{fixture::Fixture, run, TestResult};
use census_domain::model::{
    DiscoveryPriorityV1, Gender, GradYear, ProvisionalInclusionV1, RegionalMembershipV1, Sport,
};
use census_store::Store;
use std::process::Command;

#[test]
fn selected_publication_requires_explicit_store_and_rejects_ingress() -> TestResult {
    let dir = tempfile::tempdir()?;
    for args in [
        vec![
            "selected-publication",
            "--selection",
            "missing",
            "--out",
            "out",
        ],
        vec![
            "selected-publication",
            "--selection",
            "missing",
            "--out",
            "out",
            "--ingress",
            "http://localhost:8080",
        ],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_census-service"))
            .current_dir(dir.path())
            .args(args)
            .output()?;
        check!(!output.status.success());
        check!(!dir.path().join("var").exists());
        check!(!dir.path().join("out").exists());
    }
    Ok(())
}

#[tokio::test]
async fn selected_publication_refuses_legacy_and_foreign_selection_before_output() -> TestResult {
    let fixture = Fixture::create().await?;
    let legacy = fixture.directory.path().join("legacy");
    drop(Store::open(&legacy)?);
    let before = Store::format(&legacy)?;
    let out = fixture.directory.path().join("refused");
    let result = run(&legacy, &fixture.selection, &out)?;
    check!(!result.status.success());
    check!(eq; Store::format(&legacy)?, before);
    check!(!out.exists());
    let foreign = Fixture::create().await?;
    let result = run(&fixture.root, &foreign.selection, &out)?;
    check!(!result.status.success());
    check!(String::from_utf8_lossy(&result.stderr).contains("different store"));
    check!(!out.exists());
    Ok(())
}

#[tokio::test]
async fn selected_publication_refuses_unsupported_profile_and_input_before_output() -> TestResult {
    let fixture = Fixture::create().await?;
    let mut cases = Vec::new();
    let mut input = fixture.profile.input().clone();
    input
        .graduation_years
        .push(GradYear::new(2028).ok_or("year")?);
    cases.push((input, "exactly one graduation year"));
    let mut input = fixture.profile.input().clone();
    input.regional_membership = RegionalMembershipV1::SourceBackedRegion(input.source_plan.clone());
    cases.push((input, "regional membership"));
    let mut input = fixture.profile.input().clone();
    input.discovery_priority =
        DiscoveryPriorityV1::SourceBackedRegionFirst(input.source_plan.clone());
    cases.push((input, "regional priority"));
    let mut input = fixture.profile.input().clone();
    input.include_provisional = ProvisionalInclusionV1::Include;
    cases.push((input, "provisional inclusion"));
    let mut input = fixture.profile.input().clone();
    input.policies.participation = input.source_plan.clone();
    cases.push((input, "Participation"));
    let mut input = fixture.profile.input().clone();
    input.competition_categories = vec![Gender::Boys];
    cases.push((input, "outside-profile competition category"));
    let mut input = fixture.profile.input().clone();
    input.sports = vec![Sport::IndoorTrack];
    cases.push((input, "outside-profile sport"));
    for (ordinal, (input, diagnostic)) in cases.into_iter().enumerate() {
        let selection = fixture.select_profile(input, &format!("profile-{ordinal}.json"))?;
        let out = fixture.directory.path().join(format!("refused-{ordinal}"));
        let result = run(&fixture.root, &selection, &out)?;
        check!(!result.status.success());
        check!(
            String::from_utf8_lossy(&result.stderr).contains(diagnostic),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        check!(!out.exists());
    }
    Ok(())
}

#[tokio::test]
async fn selected_publication_refuses_invalid_or_unregistered_selection_before_output() -> TestResult
{
    let fixture = Fixture::create().await?;
    let out = fixture.directory.path().join("refused-selection");
    let malformed = fixture.directory.path().join("malformed.json");
    std::fs::write(&malformed, b"{not-json}")?;
    check!(!run(&fixture.root, &malformed, &out)?.status.success());
    check!(!out.exists());
    let (store, _writer) =
        Store::open_public_identity_qualification_v3(&fixture.root)?.into_parts();
    let missing = census_domain::model::ProfileSelectionV1::new(
        store.store_identity()?,
        census_domain::model::ProfileId::parse(&"b".repeat(64))?,
    );
    let missing_path = fixture.directory.path().join("missing-profile.json");
    std::fs::write(&missing_path, serde_json::to_vec(&missing)?)?;
    drop(store);
    check!(!run(&fixture.root, &missing_path, &out)?.status.success());
    check!(!out.exists());
    let old = fixture.directory.path().join("old-lifecycle");
    let result = Store::create_public_identity_qualification_v2(&old).map(|_| ());
    check!(matches!(
        result,
        Err(census_store::StoreError::PublicIdentity(
            census_store::PublicIdentityError::QualificationVersionMismatch { .. }
        ))
    ));
    check!(!old.exists());
    check!(!out.exists());
    Ok(())
}

#[tokio::test]
async fn selected_publication_refuses_every_unknown_execution_contract_before_output() -> TestResult
{
    use census_domain::profile_execution::ContractRoleV1 as Role;
    let fixture = Fixture::create().await?;
    for (ordinal, role) in Role::ALL.into_iter().enumerate() {
        let mut input = fixture.profile.input().clone();
        let foreign = census_domain::model::ContractRefV1::new(1, 9, &"f".repeat(64))?;
        match role {
            Role::Cohort => input.policies.cohort = foreign,
            Role::Event => input.policies.event = foreign,
            Role::Identity => input.policies.identity = foreign,
            Role::Affiliation => input.policies.affiliation = foreign,
            Role::Contact => input.policies.contact = foreign,
            Role::Participation => input.policies.participation = foreign,
            Role::Publication => input.policies.publication = foreign,
            Role::SourcePlan => input.source_plan = foreign,
            Role::Completion => input.completion_contract = foreign,
        }
        let selection =
            fixture.select_profile(input, &format!("unknown-contract-{ordinal}.json"))?;
        let out = fixture
            .directory
            .path()
            .join(format!("refused-contract-{ordinal}"));
        let result = run(&fixture.root, &selection, &out)?;
        check!(!result.status.success());
        let error = String::from_utf8_lossy(&result.stderr);
        check!(error.contains("unsupported exact profile contract reference"));
        check!(error.contains(&format!("{role:?}")));
        check!(!out.exists());
    }
    Ok(())
}

#[tokio::test]
async fn selected_publication_refuses_each_historical_qualification_contract_before_output(
) -> TestResult {
    use census_domain::profile_execution::{
        selected_qualification_contract_ref_v1 as old,
        selected_qualification_contract_ref_v3 as current, ContractRoleV1 as Role,
    };
    let fixture = Fixture::create().await?;
    for (ordinal, role) in Role::ALL.into_iter().enumerate() {
        let reference = old(role)?;
        if reference == current(role)? {
            check!(eq; role, Role::Participation);
            continue;
        }
        let mut input = fixture.profile.input().clone();
        match role {
            Role::Cohort => input.policies.cohort = reference,
            Role::Event => input.policies.event = reference,
            Role::Identity => input.policies.identity = reference,
            Role::Affiliation => input.policies.affiliation = reference,
            Role::Contact => input.policies.contact = reference,
            Role::Participation => input.policies.participation = reference,
            Role::Publication => input.policies.publication = reference,
            Role::SourcePlan => input.source_plan = reference,
            Role::Completion => input.completion_contract = reference,
        }
        let selection = fixture.select_profile(input, &format!("historical-{ordinal}.json"))?;
        let before = fixture.evidence_state()?;
        let out = fixture
            .directory
            .path()
            .join(format!("refused-historical-{ordinal}"));
        let result = run(&fixture.root, &selection, &out)?;
        check!(!result.status.success());
        check!(result.stdout.is_empty());
        check!(String::from_utf8_lossy(&result.stderr).contains(&format!("{role:?}")));
        check!(!out.exists());
        check!(eq; fixture.evidence_state()?, before);
    }
    Ok(())
}
