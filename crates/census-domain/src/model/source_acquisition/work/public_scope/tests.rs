use super::*;
use crate::model::{
    AcquisitionWorkV1, ProfileDate, PublicResourceObligationV1, PublicSourceResourceV1,
    RequestRepresentationV1, SchoolYear, StoreIdentity,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const OWNER: &str = "jurisdiction:MI:2026-27:1";
const ADMISSION: &str = r#"{"kind":"profile_bound_public_frontier_v1","state_key":"state","plan_fingerprint":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","selected_source":"milesplit","selected_arm":"milesplit_public_resources","durable_role":"public_resource"}"#;

fn scoped(
    owner: &str,
    revision: u32,
    manifest: &str,
    frontier: &str,
    cache: CaptureCacheIntent,
) -> TestResult<PublicResourceCaptureScopeV1> {
    Ok(PublicResourceCaptureScopeV1::new(
        owner,
        UsJurisdiction::Michigan,
        CensusRun::new(SchoolYear::DEFAULT, revision).ok_or("run")?,
        AcquisitionDigest::parse(&manifest.repeat(64))?,
        HistoryWindow::new(2023, 2026, "2026-10-21")?,
        AcquisitionDigest::parse(&frontier.repeat(64))?,
        cache,
    )?)
}

fn work(scope: &PublicResourceCaptureScopeV1) -> TestResult<AcquisitionWorkV1> {
    let target = "https://mi.milesplit.com/athletes/13749840-brennan-smith";
    Ok(AcquisitionWorkV1::for_public_resource(
        StoreIdentity::parse(&"A".repeat(64))?,
        scope,
        UsJurisdiction::Michigan,
        PublicResourceObligationV1::new(
            PublicSourceResourceV1::milesplit_athlete_profile("13749840")?,
            ProfileDate::parse("2026-10-21")?,
        ),
        RequestRepresentationV1::normalized_get(
            target,
            vec![
                ("accept-encoding".into(), "identity".into()),
                ("user-agent".into(), "Census/1.0".into()),
            ],
            "Census/1.0",
        )?,
    )?)
}

#[test]
fn scope_retains_selected_facts_and_constructs_only_the_closed_public_admission() -> TestResult {
    let scope = scoped(OWNER, 3, "a", "b", CaptureCacheIntent::AcquireNew)?;
    check!(eq; scope.owner_key(), OWNER);
    check!(eq; scope.jurisdiction(), UsJurisdiction::Michigan);
    check!(eq; scope.run().season(), SchoolYear::DEFAULT);
    check!(eq; scope.run().revision(), 3);
    check!(eq; scope.run_manifest_digest().as_str(), "a".repeat(64));
    check!(eq; scope.history().years(), 2023..=2026);
    check!(eq; scope.history().as_of().to_string(), "2026-10-21");
    check!(eq; scope.cache_intent(), CaptureCacheIntent::AcquireNew);
    check!(scope.source_admission().is_public_resources());
    check!(eq; serde_json::to_string(scope.source_admission())?, ADMISSION);
    let reopened: SourceAdmissionV1 = serde_json::from_str(ADMISSION)?;
    check!(eq; &reopened, scope.source_admission());
    Ok(())
}

#[test]
fn scope_owner_refuses_empty_controls_and_one_byte_beyond_the_exact_bound() -> TestResult {
    for owner in [
        "",
        "owner\0",
        "owner\t",
        "owner\r",
        "owner\n",
        "owner\u{7f}",
    ] {
        check!(matches!(
            scoped(owner, 1, "a", "b", CaptureCacheIntent::ReuseEligible),
            Err(error) if error.downcast_ref::<CaptureDomainError>().is_some_and(|error|
                matches!(error, CaptureDomainError::Invalid { field: "owner key" }))
        ));
    }
    for owner in ["x".repeat(65), "é".repeat(33)] {
        check!(matches!(
            scoped(&owner, 1, "a", "b", CaptureCacheIntent::ReuseEligible),
            Err(error) if error.downcast_ref::<CaptureDomainError>().is_some_and(|error|
                matches!(error, CaptureDomainError::Bound { field: "owner key", max: 64 }))
        ));
    }
    for owner in ["x".repeat(64), "é".repeat(32)] {
        let scope = scoped(&owner, 1, "a", "b", CaptureCacheIntent::ReuseEligible)?;
        check!(eq; scope.owner_key(), owner);
    }
    Ok(())
}

#[test]
fn public_work_carries_scope_facts_without_a_historical_year_or_meet_identity() -> TestResult {
    let scope = scoped(OWNER, 3, "a", "b", CaptureCacheIntent::AcquireNew)?;
    let work = work(&scope)?;
    check!(eq; work.slot().owner_key(), OWNER);
    check!(work.slot().historical_year().is_none());
    check!(work.slot().source_meet_id().is_none());
    check!(eq; work.run(), scope.run());
    check!(eq; work.run_manifest_digest(), scope.run_manifest_digest());
    check!(eq; work.history(), scope.history());
    check!(eq; work.source_admission(), scope.source_admission());
    check!(eq; work.cache_intent(), CaptureCacheIntent::AcquireNew);
    check!(eq; serde_json::to_string(work.source_admission())?, ADMISSION);
    let bytes = serde_json::to_vec(&work)?;
    let reopened: AcquisitionWorkV1 = serde_json::from_slice(&bytes)?;
    check!(eq; reopened, work);
    check!(eq; reopened.id()?, work.id()?);
    Ok(())
}

#[test]
fn cloning_retains_frontier_binding_and_a_different_frontier_changes_only_work_identity(
) -> TestResult {
    let scope = scoped(OWNER, 1, "a", "b", CaptureCacheIntent::ReuseEligible)?;
    let cloned = scope.try_clone()?;
    check!(eq; cloned, scope);
    let baseline = work(&scope)?;
    let copied = work(&cloned)?;
    check!(eq; copied, baseline);
    check!(eq; copied.id()?, baseline.id()?);
    let changed = scoped(OWNER, 1, "a", "c", CaptureCacheIntent::ReuseEligible)?;
    check!(eq; changed.source_admission().plan_fingerprint().as_str(), "c".repeat(64));
    check!(ne; changed.source_admission(), scope.source_admission());
    let changed = work(&changed)?;
    check!(eq; changed.slot(), baseline.slot());
    check!(eq; changed.slot().id()?, baseline.slot().id()?);
    check!(ne; changed.id()?, baseline.id()?);
    Ok(())
}

#[test]
fn run_manifest_and_cache_intent_remain_bound_without_resetting_the_native_slot() -> TestResult {
    let baseline = work(&scoped(
        OWNER,
        1,
        "a",
        "b",
        CaptureCacheIntent::ReuseEligible,
    )?)?;
    for scope in [
        scoped(OWNER, 2, "a", "b", CaptureCacheIntent::ReuseEligible)?,
        scoped(OWNER, 1, "c", "b", CaptureCacheIntent::ReuseEligible)?,
        scoped(OWNER, 1, "a", "b", CaptureCacheIntent::AcquireNew)?,
    ] {
        let changed = work(&scope)?;
        check!(eq; changed.slot(), baseline.slot());
        check!(eq; changed.slot().id()?, baseline.slot().id()?);
        check!(ne; changed.id()?, baseline.id()?);
    }
    Ok(())
}
