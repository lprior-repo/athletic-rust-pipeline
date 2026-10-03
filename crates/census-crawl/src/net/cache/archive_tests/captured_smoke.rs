use super::*;

#[test]
fn captured_nc_directory_and_summary_fixtures_survive_synthetic_cache_refresh() -> TestResult {
    let root = tempfile::tempdir()?;
    let captures: [(&[u8], &str, &str); 2] = [
        (
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/coach_directories/nchsaa_directory_p1.json"
            )),
            "https://maxinfosite-api-live.dragonflyathletics.com/states/NCHSAA/directory/1",
            "4cca3a67f7eecd8e143ae02e18939f7f9aefb91d8d8106ccdada87b2e72cb814",
        ),
        (
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/coach_directories/nc_staff_summary_zcum49.json"
            )),
            "https://maxinfosite-api-live.dragonflyathletics.com/schools/ZCUM49/summary",
            "ed24ab23e0a76bd8fb705bc7cd72eae771ba19bbfa4200dea498e299e8b43d2a",
        ),
    ];
    for (raw, url, expected_digest) in captures {
        check!(eq; content_digest(raw), expected_digest);
        let key = Fetcher::key_for("GET", url, "");
        let body_path = root.path().join(format!("{key}.body"));
        let meta_path = root.path().join(format!("{key}.meta.json"));
        let mut original = metadata(raw);
        original.url = url.to_owned();
        original.fetched_at = "2026-09-27".to_owned();
        original.etag = None;
        original.last_modified = None;
        write_cache(&body_path, &meta_path, raw, &original)?;
        let mut refreshed_raw = raw.to_vec();
        refreshed_raw.push(b'\n');
        let mut refreshed = original.clone();
        refreshed.content_digest = content_digest(&refreshed_raw);
        refreshed.bytes = refreshed_raw.len();
        refreshed.fetched_at = "fixture-refresh-not-live-acquisition".to_owned();
        write_cache(&body_path, &meta_path, &refreshed_raw, &refreshed)?;
        assert_capture(root.path(), raw, &original)?;
        assert_capture(root.path(), &refreshed_raw, &refreshed)?;
        let (served, body) = read_cache(&body_path, &meta_path)?.ok_or("hit")?;
        check!(eq; body, refreshed_raw);
        check!(eq; served.url, url);
        check!(eq; served.fetched_at, refreshed.fetched_at);
        check!(ne; served.content_digest, expected_digest);
        let original_json: serde_json::Value = serde_json::from_slice(raw)?;
        let refreshed_json: serde_json::Value = serde_json::from_slice(&body)?;
        check!(eq; original_json, refreshed_json);
    }
    Ok(())
}
