use super::*;
use crate::model::{Evidence, Gender, Id, SchoolId, SchoolYear, SourceRef, Sport, TeamId};
use crate::UsJurisdiction;
use std::error::Error;

mod boundaries;
mod co_ops;
mod fixtures;
mod profiles;
mod temporal;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn date(value: &str) -> TestResult<ProfileDate> {
    Ok(ProfileDate::parse(value)?)
}

fn range(from: &str, through: &str) -> TestResult<ProfileDateRangeV1> {
    Ok(ProfileDateRangeV1::new(date(from)?, date(through)?)?)
}

fn context(sport: Sport, category: Gender, year: i16) -> TestResult<RegionContextV1> {
    let school_year = SchoolYear::new(year)
        .ok_or_else(|| std::io::Error::other("invalid fixture school year"))?;
    Ok(RegionContextV1 {
        sport,
        category,
        school_year,
    })
}

fn xc_boys() -> TestResult<RegionContextV1> {
    context(Sport::CrossCountry, Gender::Boys, 2026)
}

fn school(label: &str, jurisdiction: UsJurisdiction) -> TestResult<RegionSchoolV1> {
    Ok(RegionSchoolV1::new(
        Id::mint("sch", &["synthetic-region-fixture", label]),
        jurisdiction,
    )?)
}

fn program(label: &str) -> TeamId {
    Id::mint("team", &["synthetic-region-fixture", label])
}

fn evidence(label: &str) -> TestResult<RegionEvidenceV1> {
    let source = SourceRef::new(
        "synthetic-public-authority",
        Some("https://example.invalid/region-fixture.json".to_string()),
    );
    Ok(RegionEvidenceV1::new(RegionEvidenceInputV1 {
        evidence: Evidence::parsed(source, "2026-10-01T12:00:00Z"),
        capture_sha256: "a".repeat(64),
        locator: format!("synthetic-row:{label}"),
        statement: format!("Synthetic regional assignment statement {label}"),
    })?)
}

fn assertion(
    subject: RegionSubjectV1,
    statement: RegionStatementV1,
    label: &str,
) -> TestResult<RegionAssertionV1> {
    Ok(RegionAssertionV1::new(RegionAssertionInputV1 {
        subject,
        context: xc_boys()?,
        effective: RegionEffectiveV1::Inclusive(range("2026-08-01", "2027-07-31")?),
        statement,
        provenance: evidence(label)?,
    }))
}

fn contract(assertions: Vec<RegionAssertionV1>) -> TestResult<RegionMembershipContractV1> {
    Ok(RegionMembershipContractV1::new(
        RegionMembershipContractInputV1 {
            version: 1,
            revision: 7,
            region: RegionKeyV1::parse("synthetic:region-fixture")?,
            jurisdictions: vec![UsJurisdiction::Michigan],
            effective: range("2026-01-01", "2027-12-31")?,
            assertions,
        },
    )?)
}

fn query(
    target: RegionMembershipTargetV1,
    effective_on: &str,
) -> TestResult<RegionMembershipQueryV1> {
    Ok(RegionMembershipQueryV1::new(
        target,
        xc_boys()?,
        date(effective_on)?,
    )?)
}

fn replace_assertion_context(
    assertion: &RegionAssertionV1,
    context: RegionContextV1,
) -> TestResult<RegionAssertionV1> {
    let mut input = assertion.input().clone();
    input.context = context;
    Ok(RegionAssertionV1::new(input))
}
