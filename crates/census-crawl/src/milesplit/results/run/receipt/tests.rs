use super::*;
use crate::milesplit::owned::{parse_owned_meet, OwnedMeetOutcome};
use census_domain::model::{CanonicalSchool, SourceIdentity, SourceNamespace};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const OWNED: &[u8] = include_bytes!("../../../owned/fixtures/troy_725218.json");
const RAW: &[u8] = include_bytes!(
    "../../../../../tests/fixtures/milesplit/troy_725218_rs1266814_raw_projection.html"
);

fn captured(url: String, body: &[u8]) -> FetchOutcome {
    FetchOutcome {
        url,
        response_url: None,
        method: "GET".into(),
        status: 200,
        content_digest: crate::net::cache::content_digest(body),
        bytes: body.len(),
        fetched_at: "2026-10-01T23:44:16Z".into(),
        from_cache: false,
        content_type: None,
        body: body.to_vec(),
    }
}

fn receipt(
    owned: FetchOutcome,
    metadata: &FetchOutcome,
    schools: &[CanonicalSchool],
) -> TestResult<(String, serde_json::Value)> {
    let reference =
        ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266814/raw")
            .ok_or("owned source unit")?;
    let acquired = AcquiredMeet::new(OwnedMeetOutcome {
        verdict: parse_owned_meet(&owned.body, 725218),
        capture: owned,
    })?;
    let page = super::super::metadata::parse_capture(metadata, &reference)?;
    let mut stats = super::super::super::Stats::default();
    let bindings = super::super::ProviderSchools::from_schools(schools);
    let performance_as_of = chrono::NaiveDate::parse_from_str("2026-10-07", "%Y-%m-%d")?;
    let input = Input {
        acquired: &acquired,
        reference: &reference,
        page: &page,
        schools: &bindings,
        performance_as_of,
    };
    let result_set = acquired
        .result_set(&reference.rsid, performance_as_of)
        .ok_or("result set")?;
    let complete = result_set.page.individual_parse_complete();
    let mut projected =
        match super::super::projection::prepare(&input, result_set.indices, &mut stats)? {
            super::super::projection::Prepared::Complete(projected) => projected,
            super::super::projection::Prepared::Unfinished { error, .. } => {
                return Err(error.into())
            }
        };
    super::super::evidence::bind(
        &mut projected,
        &reference,
        &page,
        &acquired.outcome.capture,
        metadata,
    )?;
    identify_retained(&mut projected)?;
    Ok(projection(
        &input,
        metadata,
        &projected,
        complete && stats.rows_without_school == 0,
        stats.rows,
    )?)
}

fn captures() -> TestResult<(FetchOutcome, FetchOutcome)> {
    let reference =
        ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266814/raw")
            .ok_or("owned source unit")?;
    Ok((
        captured(crate::milesplit::fetch::owned_meet_url(&reference)?, OWNED),
        captured(reference.url, RAW),
    ))
}

fn school(name: &str, team: &str) -> CanonicalSchool {
    let mut school = CanonicalSchool::new(
        census_domain::UsJurisdiction::Alabama,
        name,
        census_domain::model::normalize_name(name),
        None,
    )
    .0;
    school
        .source_identities
        .push(SourceIdentity::new(SourceNamespace::MilesplitSchool, team));
    school
}

#[test]
fn cached_transport_replay_keeps_capture_bound_projection_identity_and_payload() -> TestResult {
    let (mut owned, mut metadata) = captures()?;
    let first = receipt(owned.clone(), &metadata, &[])?;
    owned.from_cache = true;
    metadata.from_cache = true;
    check!(eq; receipt(owned, &metadata, &[])?, first);
    Ok(())
}

#[test]
fn independently_reacquired_metadata_keeps_distinct_honest_projection_provenance() -> TestResult {
    let (owned, mut metadata) = captures()?;
    let first = receipt(owned.clone(), &metadata, &[])?;
    metadata.fetched_at = "2026-10-02T00:00:00Z".into();
    let next = receipt(owned, &metadata, &[])?;
    check!(ne; next.0, first.0);
    check!(eq;
        next.1["raw_metadata_capture"]["content_digest"],
        first.1["raw_metadata_capture"]["content_digest"]
    );
    check!(eq;
        next.1["raw_metadata_capture"]["response_url"],
        serde_json::Value::Null
    );
    check!(eq;
        next.1["raw_metadata_capture"]["fetched_at"],
        metadata.fetched_at
    );
    Ok(())
}

#[test]
fn changed_exact_school_owner_cannot_reuse_a_completed_capture_projection_receipt() -> TestResult {
    let (owned, metadata) = captures()?;
    let first = receipt(
        owned.clone(),
        &metadata,
        &[school("Spann", "38332"), school("Charles", "4912")],
    )?;
    let next = receipt(
        owned,
        &metadata,
        &[
            school("Different canonical school", "38332"),
            school("Charles", "4912"),
        ],
    )?;
    check!(eq; first.1["disposition"], "projection_applied");
    check!(eq; next.1["disposition"], "projection_applied");
    check!(eq; next.1["capture"], first.1["capture"]);
    check!(eq;
        next.1["raw_metadata_capture"],
        first.1["raw_metadata_capture"]
    );
    check!(ne;
        next.1["projection_context_digest"],
        first.1["projection_context_digest"]
    );
    check!(ne; next.0, first.0);
    Ok(())
}

#[test]
fn observed_metadata_response_location_is_distinct_from_historical_unknown_location() -> TestResult
{
    let (owned, mut metadata) = captures()?;
    let first = receipt(owned.clone(), &metadata, &[])?;
    metadata.response_url = Some(metadata.url.clone());
    let observed = receipt(owned, &metadata, &[])?;
    check!(ne; observed.0, first.0);
    check!(eq;
        observed.1["raw_metadata_capture"]["content_digest"],
        first.1["raw_metadata_capture"]["content_digest"]
    );
    check!(eq;
        observed.1["raw_metadata_capture"]["fetched_at"],
        first.1["raw_metadata_capture"]["fetched_at"]
    );
    check!(eq;
        first.1["raw_metadata_capture"]["response_url"],
        serde_json::Value::Null
    );
    check!(eq;
        observed.1["raw_metadata_capture"]["response_url"],
        metadata.url
    );
    Ok(())
}

#[test]
fn contradictory_observed_owned_response_refuses_projection_of_matching_body_rows() -> TestResult {
    let (mut owned, metadata) = captures()?;
    owned.response_url = Some("https://al.milesplit.com/api/v1/meets/770621/performances".into());
    let reference =
        ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266814/raw")
            .ok_or("Troy reference")?;
    let acquired = AcquiredMeet::new(OwnedMeetOutcome {
        verdict: parse_owned_meet(&owned.body, 725218),
        capture: owned,
    })?;
    let page = super::super::metadata::parse_capture(&metadata, &reference)?;
    let mut stats = super::super::super::Stats::default();
    let schools = super::super::ProviderSchools::default();
    let performance_as_of = chrono::NaiveDate::parse_from_str("2026-10-07", "%Y-%m-%d")?;
    let input = Input {
        acquired: &acquired,
        reference: &reference,
        page: &page,
        schools: &schools,
        performance_as_of,
    };
    let indices = acquired
        .result_set(&reference.rsid, performance_as_of)
        .ok_or("result set")?
        .indices;
    match super::super::projection::prepare(&input, indices, &mut stats) {
        Err(crate::CrawlError::Schema { url, .. }) => {
            check!(eq; url, reference.url);
        }
        outcome => {
            return Err(format!("foreign observed owner cannot bind the body: {outcome:?}").into())
        }
    }
    check!(eq; stats.rows, 0);
    check!(eq; acquired.outcome.capture.body, OWNED);
    Ok(())
}
