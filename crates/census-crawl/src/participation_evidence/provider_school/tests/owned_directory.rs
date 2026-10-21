use super::*;
use crate::net::FetchOutcome;
use census_domain::model::AcquisitionDigest;

fn owned_capture() -> FetchOutcome {
    FetchOutcome {
        url: DIRECTORY_URL.into(),
        response_url: Some(DIRECTORY_URL.into()),
        method: "GET".into(),
        status: 200,
        content_digest: hash(DIRECTORY.as_bytes()),
        bytes: DIRECTORY.len(),
        fetched_at: "2026-10-16T02:58:00Z".into(),
        from_cache: false,
        content_type: Some("text/html; charset=UTF-8".into()),
        body: DIRECTORY.as_bytes().to_vec(),
    }
}

#[test]
fn owned_helper_preserves_original_directory_projection_and_current_kernel_identity() -> TestResult
{
    let original = packet(DIRECTORY, ROSTER)?.read()?;
    let (team, proof, kernel) = read_directory_roster_child(&owned_capture(), "12439")?;
    check!(eq; team.id, "12439");
    check!(eq; team.url, TEAM_URL);
    check!(eq; serde_json::to_value(proof)?,
        serde_json::to_value(original.directory_proof())?);
    let current = crate::participation_evidence::roster_source_issuance::
        current_roster_semantic_kernel_digest()?;
    check!(eq; kernel, AcquisitionDigest::parse(current)?);
    Ok(())
}

#[test]
fn owned_helper_refuses_repaired_hash_invalid_utf8_and_unknown_transport() -> TestResult {
    let mut capture = owned_capture();
    capture.body = vec![0xff];
    capture.bytes = capture.body.len();
    capture.content_digest = hash(&capture.body);
    check!(read_directory_roster_child(&capture, "12439").is_err());
    for content_type in [
        None,
        Some("application/json"),
        Some("text/html; charset=iso-8859-1"),
    ] {
        let mut capture = owned_capture();
        capture.content_type = content_type.map(str::to_owned);
        check!(read_directory_roster_child(&capture, "12439").is_err());
    }
    let mut capture = owned_capture();
    capture.content_digest = "0".repeat(64);
    check!(read_directory_roster_child(&capture, "12439").is_err());
    capture = owned_capture();
    capture.bytes = capture.bytes.checked_add(1).ok_or("body length")?;
    check!(read_directory_roster_child(&capture, "12439").is_err());
    Ok(())
}

#[test]
fn owned_helper_refuses_foreign_url_status_method_and_clocks() -> TestResult {
    let mut capture = owned_capture();
    capture.response_url = Some("https://co.milesplit.com/teams".into());
    check!(read_directory_roster_child(&capture, "12439").is_err());
    capture = owned_capture();
    capture.status = 403;
    check!(read_directory_roster_child(&capture, "12439").is_err());
    capture = owned_capture();
    capture.method = "POST".into();
    check!(read_directory_roster_child(&capture, "12439").is_err());
    for clock in ["not-a-date", "2026-10-16T03:58:00+01:00"] {
        let mut capture = owned_capture();
        capture.fetched_at = clock.into();
        check!(read_directory_roster_child(&capture, "12439").is_err());
    }
    Ok(())
}
