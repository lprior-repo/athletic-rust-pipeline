use super::build;
use census_domain::model::{serialized_digest, CaptureCacheIntent};
use census_reconcile::identity::WorkflowIdentity;
use fixture::{digest, Fixture, TestResult};

mod fixture;
mod refusals;

#[test]
fn five_state_facts_bind_exact_original_request_and_canonical_owner_order() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.facts.reverse();
    let before = fixture.witness()?;
    let binding = fixture.binding()?;
    let mut expected = fixture::sites();
    expected.sort_unstable();
    check!(eq; binding.owners.iter().map(|owner| owner.site).collect::<Vec<_>>(), expected);
    check!(eq; binding.version, 1);
    check!(eq; &binding.store, fixture.bound.export_job.require_selected_publication().store_identity());
    check!(eq; binding.run, fixture.manifest.run);
    check!(eq; &binding.selection, fixture.bound.export_job.require_selected_publication());
    check!(eq; &binding.capture_declaration_digest, &fixture.capture);
    check!(eq; binding.run_manifest_digest, digest(&fixture.manifest)?);
    check!(eq; binding.national_request_digest, digest(&fixture.bound)?);
    check!(eq; binding.national_owner_key, WorkflowIdentity::national(
        fixture.bound.season, fixture.bound.revision, &fixture.bound.jurisdictions).to_string());
    for owner in &binding.owners {
        let facts = fixture
            .facts
            .iter()
            .find(|facts| facts.site == owner.site)
            .ok_or("owner facts")?;
        let plan = facts.state.plan.as_ref().ok_or("source plan")?;
        check!(eq; owner.owner_key, facts.state.identity);
        check!(eq; owner.jurisdiction_request_digest, digest(&fixture.bound.for_jurisdiction(owner.site))?);
        check!(eq; owner.source_plan_fingerprint.as_str(), plan.fingerprint.as_str());
        check!(eq; owner.history, fixture.bound.history);
        check!(eq; owner.cache_intent, CaptureCacheIntent::ReuseEligible);
        check!(eq; &owner.directory_request, &facts.directory_request);
        check!(eq; &owner.robots_request, &facts.robots_request);
    }
    check!(eq; fixture.witness()?, before);
    fixture.facts.rotate_left(2);
    check!(eq; fixture.binding()?, binding);
    Ok(())
}

#[test]
fn bound_request_order_is_preserved_in_digest_while_owner_order_is_canonical() -> TestResult {
    let mut fixture = Fixture::new()?;
    let original = fixture.binding()?;
    fixture.bound.jurisdictions.reverse();
    let changed = fixture.binding()?;
    check!(eq; changed.national_owner_key, original.national_owner_key);
    check!(eq; changed.owners, original.owners);
    check!(eq; changed.national_request_digest, digest(&fixture.bound)?);
    check!(ne; changed.national_request_digest, original.national_request_digest);
    check!(ne; serialized_digest(&changed)?, serialized_digest(&original)?);
    Ok(())
}

#[test]
fn full_national_request_changes_are_bound_even_when_child_requests_are_identical() -> TestResult {
    let mutations: [fn(&mut crate::restate_services::wire::NationalRequest) -> TestResult; 3] = [
        |bound| {
            bound.pass_budget += 1;
            Ok(())
        },
        |bound| {
            bound.pass_delay_seconds += 1;
            Ok(())
        },
        |bound| {
            bound.generation = Some(crate::restate_services::ExportGeneration::parse(
                "another-generation",
            )?);
            Ok(())
        },
    ];
    for mutate in mutations {
        let mut fixture = Fixture::new()?;
        let original = fixture.binding()?;
        mutate(&mut fixture.bound)?;
        let changed = fixture.binding()?;
        check!(eq; changed.owners, original.owners);
        check!(eq; changed.selection, original.selection);
        check!(eq; changed.national_owner_key, original.national_owner_key);
        check!(ne; changed.national_request_digest, original.national_request_digest);
        check!(eq; changed.national_request_digest, digest(&fixture.bound)?);
    }
    Ok(())
}

#[test]
fn changed_valid_retained_plan_fingerprint_is_a_distinct_declaration_binding() -> TestResult {
    let mut fixture = Fixture::new()?;
    let original = fixture.binding()?;
    let site = fixture.first_mut()?.site;
    let fingerprint = crate::restate_services::plan::compute_plan_fingerprint(
        site,
        fixture.bound.season,
        fixture.bound.revision,
        crate::restate_services::plan::BrowserLaneState::Configured,
    );
    fixture.plan_mut()?.fingerprint = fingerprint.clone();
    let changed = fixture.binding()?;
    check!(ne; &changed, &original);
    check!(ne; serialized_digest(&changed)?, serialized_digest(&original)?);
    check!(eq; changed.national_request_digest, original.national_request_digest);
    for (after, before) in changed.owners.iter().zip(&original.owners) {
        if after.site == site {
            let mut expected = before.clone();
            expected.source_plan_fingerprint =
                census_domain::model::AcquisitionDigest::parse(&fingerprint)?;
            check!(eq; after, &expected);
        } else {
            check!(eq; after, before);
        }
    }
    Ok(())
}

#[test]
fn refresh_binds_acquire_new_and_exact_updated_child_requests() -> TestResult {
    let mut fixture = Fixture::new()?;
    let original = fixture.binding()?;
    fixture.bound.refresh = true;
    let changed = build(
        &fixture.manifest,
        &fixture.capture,
        &fixture.bound,
        &fixture.facts,
    )?;
    for owner in &changed.owners {
        check!(eq; owner.cache_intent, CaptureCacheIntent::AcquireNew);
        check!(eq; owner.jurisdiction_request_digest, digest(&fixture.bound.for_jurisdiction(owner.site))?);
    }
    check!(ne; changed.national_request_digest, original.national_request_digest);
    check!(eq; changed.selection, original.selection);
    Ok(())
}
