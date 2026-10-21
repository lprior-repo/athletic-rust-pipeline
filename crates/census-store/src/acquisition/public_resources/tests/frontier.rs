use super::super::*;
use super::fixtures::*;
use census_domain::model::*;
use census_domain::UsJurisdiction;

#[test]
fn binding_changes_preserve_slot_budget_identity_and_refuse_declaration_replay() -> TestResult {
    let f = fixture("2026-10-21")?;
    let held = entry(&f.record, PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    let receipt = f.record.receipt()?;
    let other_day = fixture("2026-10-20")?;
    let mut plan = f.record.binding.clone();
    plan.owners
        .first_mut()
        .ok_or("fixture owner")?
        .source_plan_fingerprint = AcquisitionDigest::parse(&"b".repeat(64))?;
    let mut profile = f.profile.input().clone();
    profile.include_provisional = ProvisionalInclusionV1::Include;
    let profile = QueryProfileV1::new(profile)?;
    let mut selection = f.record.binding.clone();
    selection.selection = ProfileSelectionV1::new(selection.store.clone(), profile.id()?);
    selection.validate(&f.capture, &f.manifest, &profile)?;
    let mut history = f.record.binding.clone();
    history.owners.first_mut().ok_or("fixture owner")?.history =
        HistoryWindow::new(2022, 2026, "2026-10-21")?;
    for changed in [other_day.record.binding, plan, selection, history] {
        let record = PublicResourceDeclarationV1::new(changed, f.record.admitted_at)?;
        let candidate = entry(&record, PublicResourceRootKindV1::MileSplitTeamDirectory)?;
        equal(candidate.work.slot().id()?, held.work.slot().id()?)?;
        equal(candidate.work.id()? == held.work.id()?, false)?;
        refusal(
            f.record.replay(&record.binding, &receipt),
            "public resource declaration binding conflict",
        )?;
        refusal(held.validate(&record), "public resource frontier identity")?;
        refusal(
            candidate.validate(&f.record),
            "public resource frontier identity",
        )?;
    }
    Ok(())
}

#[test]
fn root_kinds_have_distinct_slots_and_cannot_exchange_bound_requests() -> TestResult {
    let f = fixture("2026-10-21")?;
    let directory = entry(&f.record, PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    let robots = entry(&f.record, PublicResourceRootKindV1::Robots)?;
    equal(
        directory.work.slot().id()? == robots.work.slot().id()?,
        false,
    )?;
    equal(directory.id()? == robots.id()?, false)?;
    refusal(
        PublicResourceFrontierEntryV1::new(
            &f.record,
            UsJurisdiction::Michigan,
            PublicResourceRootKindV1::MileSplitTeamDirectory,
            robots.work.request().clone(),
        ),
        "public resource frontier request differs",
    )?;
    let mut forged = directory.clone();
    forged.kind = PublicResourceRootKindV1::Robots;
    refusal(
        forged.validate(&f.record),
        "public resource frontier identity",
    )
}

#[test]
fn borrowed_work_entry_digest_matches_retained_frontier_for_each_root() -> TestResult {
    let f = fixture("2026-10-21")?;
    for kind in [
        PublicResourceRootKindV1::MileSplitTeamDirectory,
        PublicResourceRootKindV1::Robots,
    ] {
        let held = entry(&f.record, kind)?;
        equal(
            super::super::frontier::entry_id(&f.record, &held.work)?,
            held.id()?,
        )?;
        equal(super::super::frontier::root_kind(&held.work)?, kind)?;
    }
    Ok(())
}

#[test]
fn same_url_with_changed_normalized_headers_is_not_the_bound_root_request() -> TestResult {
    let f = fixture("2026-10-21")?;
    let owner = f.record.binding.owner(UsJurisdiction::Michigan)?;
    let mut headers = owner.directory_request.representation_headers().to_vec();
    headers.first_mut().ok_or("fixture accept header")?.1 = "text/html".to_string();
    let changed = RequestRepresentationV1::normalized_get(
        owner.directory_request.original_url(),
        headers,
        owner.directory_request.effective_user_agent(),
    )?;
    refusal(
        PublicResourceFrontierEntryV1::new(
            &f.record,
            owner.site,
            PublicResourceRootKindV1::MileSplitTeamDirectory,
            changed,
        ),
        "public resource frontier request differs",
    )
}

#[test]
fn frontier_digest_and_exact_work_metadata_are_bound_to_the_declaration() -> TestResult {
    let f = fixture("2026-10-21")?;
    let held = entry(&f.record, PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    let mut forged = held.clone();
    forged.plan_digest = AcquisitionDigest::parse(&"b".repeat(64))?;
    refusal(
        forged.validate(&f.record),
        "public resource frontier identity",
    )?;
    let mut changed = f.record.clone();
    let owner = changed
        .binding
        .owners
        .iter_mut()
        .find(|owner| owner.site == UsJurisdiction::Michigan)
        .ok_or("fixture Michigan owner")?;
    owner.cache_intent = CaptureCacheIntent::AcquireNew;
    let candidate = entry(&changed, PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    let mut forged = candidate;
    forged.declaration_digest = held.declaration_digest.clone();
    forged.plan_digest = held.plan_digest.clone();
    equal(forged.work.slot().id()?, held.work.slot().id()?)?;
    refusal(
        forged.validate(&f.record),
        "public resource frontier work binding",
    )
}
