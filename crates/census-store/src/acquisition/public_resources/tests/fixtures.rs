use super::super::*;
use crate::{Receipt, StoreError, StoreResult};
use census_domain::model::*;
use census_domain::UsJurisdiction;
use chrono::{DateTime, Utc};
use std::fmt::Debug;

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub(super) struct Fixture {
    pub(super) record: PublicResourceDeclarationV1,
    pub(super) capture: CaptureQualification,
    pub(super) manifest: RunManifest,
    pub(super) profile: QueryProfileV1,
}

pub(super) fn equal<T: Debug + PartialEq>(actual: T, expected: T) -> TestResult {
    if actual != expected {
        return Err(format!("actual={actual:?}; expected={expected:?}").into());
    }
    Ok(())
}

pub(super) fn refusal<T: Debug>(result: StoreResult<T>, expected: &str) -> TestResult {
    match result {
        Err(StoreError::Refused { detail }) => equal(detail.as_str(), expected),
        other => Err(format!("expected refusal {expected:?}; actual={other:?}").into()),
    }
}

pub(super) fn at(value: &str) -> TestResult<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc))
}

pub(super) fn request(url: &str) -> TestResult<RequestRepresentationV1> {
    Ok(RequestRepresentationV1::normalized_get(
        url,
        vec![
            ("accept".to_string(), "*/*".to_string()),
            ("accept-encoding".to_string(), "identity".to_string()),
            ("user-agent".to_string(), "Census/1.0".to_string()),
        ],
        "Census/1.0",
    )?)
}

pub(super) fn fixture(as_of: &str) -> TestResult<Fixture> {
    let identity = StoreIdentity::parse(&"1".repeat(64))?;
    let profile = super::profiles::profile(as_of)?;
    let manifest = RunManifest {
        store_identity: identity.as_str().to_string(),
        run: CensusRun::new(SchoolYear::DEFAULT, 1).ok_or("fixture run")?,
        cohort: GradYear::CO2027,
        jurisdictions: profile.input().jurisdictions.clone(),
    };
    let capture = CaptureQualification::V2(CaptureQualificationV2::new(identity.clone()));
    let mut owners = Vec::new();
    for site in &manifest.jurisdictions {
        owners.push(owner(*site, as_of)?);
    }
    let binding = PublicResourceDeclarationBindingV1 {
        version: 1,
        store: identity.clone(),
        run: manifest.run,
        capture_declaration_digest: codec::object_digest(&capture)?,
        run_manifest_digest: codec::object_digest(&manifest)?,
        selection: ProfileSelectionV1::new(identity, profile.id()?),
        national_owner_key: "national-public-resource-fixture".to_string(),
        national_request_digest: AcquisitionDigest::parse(&"2".repeat(64))?,
        owners,
    };
    let record = PublicResourceDeclarationV1::new(binding, at("2026-10-21T08:00:00Z")?)?;
    record.binding.validate(&capture, &manifest, &profile)?;
    Ok(Fixture {
        record,
        capture,
        manifest,
        profile,
    })
}

fn owner(site: UsJurisdiction, as_of: &str) -> TestResult<PublicResourceOwnerBindingV1> {
    let origin = format!("https://{}.milesplit.com", site.code().to_ascii_lowercase());
    Ok(PublicResourceOwnerBindingV1 {
        owner_key: format!("public-resource-{}", site.code()),
        site,
        jurisdiction_request_digest: AcquisitionDigest::parse(&"3".repeat(64))?,
        source_plan_fingerprint: AcquisitionDigest::parse(&"4".repeat(64))?,
        history: HistoryWindow::new(2023, 2026, as_of)?,
        cache_intent: CaptureCacheIntent::ReuseEligible,
        directory_request: request(&format!("{origin}/teams"))?,
        robots_request: request(&format!("{origin}/robots.txt"))?,
    })
}

pub(super) fn entry(
    record: &PublicResourceDeclarationV1,
    kind: PublicResourceRootKindV1,
) -> TestResult<PublicResourceFrontierEntryV1> {
    let owner = record.binding.owner(UsJurisdiction::Michigan)?;
    let request = match kind {
        PublicResourceRootKindV1::MileSplitTeamDirectory => &owner.directory_request,
        PublicResourceRootKindV1::Robots => &owner.robots_request,
    };
    Ok(PublicResourceFrontierEntryV1::new(
        record,
        owner.site,
        kind,
        request.clone(),
    )?)
}

pub(super) fn entry_receipt(
    record: &PublicResourceDeclarationV1,
    entry: &PublicResourceFrontierEntryV1,
) -> TestResult<Receipt> {
    Ok(Receipt {
        operation: format!("{FRONTIER_OPERATION_PREFIX}{}", entry.id()?),
        digest: codec::object_digest(entry)?.as_str().to_string(),
        at: record.admitted_at.to_rfc3339(),
        appended: 0,
        credited: false,
    })
}
