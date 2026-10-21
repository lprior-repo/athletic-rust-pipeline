use super::super::*;
use super::fixtures::*;
use census_domain::model::*;
use census_domain::UsJurisdiction;

#[test]
fn canonical_owner_inventory_accepts_manifest_order_without_reordering_owners() -> TestResult {
    let mut f = fixture("2026-10-21")?;
    let held_owners = f.record.binding.owners.clone();
    f.manifest.jurisdictions.reverse();
    f.record.binding.run_manifest_digest = codec::object_digest(&f.manifest)?;
    f.record
        .binding
        .validate(&f.capture, &f.manifest, &f.profile)?;
    equal(f.record.binding.owners, held_owners)
}

#[test]
fn each_root_requires_exact_site_url_and_normalized_request_recipe() -> TestResult {
    let f = fixture("2026-10-21")?;
    let held = f.record.binding.owner(UsJurisdiction::Michigan)?;
    for url in [
        "https://mi.milesplit.com/teams/",
        "https://mi.milesplit.com/teams?year=2026",
        "https://co.milesplit.com/teams",
        "http://mi.milesplit.com/teams",
    ] {
        let mut owner = held.clone();
        owner.directory_request = request(url)?;
        refusal(
            owner.validate_requests(),
            "public resource bound root request",
        )?;
    }
    for url in [
        "https://co.milesplit.com/robots.txt",
        "https://mi.milesplit.com/robots.txt/",
    ] {
        let mut owner = held.clone();
        owner.robots_request = request(url)?;
        refusal(
            owner.validate_requests(),
            "public resource bound root request",
        )?;
    }
    let mut owner = held.clone();
    owner.directory_request =
        RequestRepresentationV1::get("https://mi.milesplit.com/teams", Vec::new(), "Census/1.0")?;
    refusal(
        owner.validate_requests(),
        "public resource bound root request",
    )
}

#[test]
fn foreign_store_capture_profile_and_rehashed_manifest_refuse_distinctly() -> TestResult {
    let f = fixture("2026-10-21")?;
    let foreign = StoreIdentity::parse(&"b".repeat(64))?;
    let capture = CaptureQualification::V2(CaptureQualificationV2::new(foreign.clone()));
    refusal(
        f.record.binding.validate(&capture, &f.manifest, &f.profile),
        "public resource declaration lineage",
    )?;
    let mut binding = f.record.binding.clone();
    binding.selection = ProfileSelectionV1::new(foreign, f.profile.id()?);
    refusal(
        binding.validate(&f.capture, &f.manifest, &f.profile),
        "public resource selected profile store",
    )?;
    let other_profile = super::profiles::profile("2026-10-20")?;
    refusal(
        f.record
            .binding
            .validate(&f.capture, &f.manifest, &other_profile),
        "public resource selected profile binding",
    )?;
    let mut manifest = f.manifest.clone();
    *manifest
        .jurisdictions
        .first_mut()
        .ok_or("fixture jurisdiction")? = UsJurisdiction::Ohio;
    let mut binding = f.record.binding.clone();
    binding.run_manifest_digest = codec::object_digest(&manifest)?;
    refusal(
        binding.validate(&f.capture, &manifest, &f.profile),
        "public resource owner scope",
    )
}

#[test]
fn owner_order_duplicate_site_and_duplicate_key_are_closed() -> TestResult {
    let f = fixture("2026-10-21")?;
    let mut reversed = f.record.binding.clone();
    reversed.owners.reverse();
    refusal(
        reversed.validate_shape(),
        "public resource duplicate or unordered owner",
    )?;
    let mut duplicate_site = f.record.binding.clone();
    let first = duplicate_site
        .owners
        .first()
        .ok_or("fixture first owner")?
        .clone();
    *duplicate_site
        .owners
        .get_mut(1)
        .ok_or("fixture second owner")? = first;
    refusal(
        duplicate_site.validate_shape(),
        "public resource duplicate or unordered owner",
    )?;
    let mut duplicate_key = f.record.binding.clone();
    let key = duplicate_key
        .owners
        .first()
        .ok_or("fixture first owner")?
        .owner_key
        .clone();
    duplicate_key
        .owners
        .get_mut(1)
        .ok_or("fixture second owner")?
        .owner_key = key;
    refusal(
        duplicate_key.validate_shape(),
        "public resource duplicate or unordered owner",
    )?;
    let mut incomplete = f.record.binding.clone();
    incomplete.owners.pop().ok_or("fixture owner to remove")?;
    refusal(
        incomplete.validate(&f.capture, &f.manifest, &f.profile),
        "public resource owner scope",
    )
}

#[test]
fn profile_as_of_and_exact_capture_protocol_cannot_be_rebound() -> TestResult {
    let f = fixture("2026-10-21")?;
    let mut binding = f.record.binding.clone();
    binding.owners.first_mut().ok_or("fixture owner")?.history =
        HistoryWindow::new(2023, 2026, "2026-10-20")?;
    refusal(
        binding.validate(&f.capture, &f.manifest, &f.profile),
        "public resource owner scope",
    )?;
    let legacy = CaptureQualification::V1(CaptureQualificationV1::new(binding.store.clone()));
    binding.capture_declaration_digest = codec::object_digest(&legacy)?;
    refusal(
        binding.validate(&legacy, &f.manifest, &f.profile),
        "public resource declaration lineage",
    )
}
