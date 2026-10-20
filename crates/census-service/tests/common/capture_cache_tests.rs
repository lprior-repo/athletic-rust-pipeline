use super::*;

#[test]
fn captured_response_preserves_legacy_metadata_presence_and_body_bytes() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let url = "https://example.test/capture";
    let prefix = directory.path().join("689b8d061c5ff38d4650a942126d5cb5");
    let expected = json!({
        "url": url, "method": "GET", "status": 200,
        "content_digest": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        "bytes": 3, "fetched_at": "2026-09-20T00:00:00Z",
    });
    for (response_url, extra) in [
        (ResponseUrl::Absent, json!({})),
        (ResponseUrl::Null, json!({"response_url": null})),
        (ResponseUrl::Url(url), json!({"response_url": url})),
    ] {
        seed_response(
            directory.path(),
            url,
            b"abc",
            "2026-09-20T00:00:00Z",
            Metadata {
                response_url,
                ..Metadata::default()
            },
        )?;
        let actual: serde_json::Value =
            serde_json::from_slice(&std::fs::read(prefix.with_extension("meta.json"))?)?;
        let mut expected = expected.clone();
        expected
            .as_object_mut()
            .context("metadata object")?
            .extend(extra.as_object().context("extra fields")?.clone());
        anyhow::ensure!(actual == expected, "fixture metadata changed: {actual}");
        anyhow::ensure!(std::fs::read(prefix.with_extension("body"))? == b"abc");
    }
    seed(directory.path(), url, b"abc", "2026-09-20T00:00:00Z", &[])?;
    let actual: serde_json::Value =
        serde_json::from_slice(&std::fs::read(prefix.with_extension("meta.json"))?)?;
    let mut expected = expected;
    expected["representation"] = json!([]);
    anyhow::ensure!(
        actual == expected,
        "empty representation must remain present"
    );
    Ok(())
}

#[test]
fn captured_response_keeps_token_identity_and_content_type() -> Result<()> {
    let directory = tempfile::tempdir()?;
    seed_response(
        directory.path(),
        "https://example.test/capture",
        b"abc",
        "2026-09-22T12:00:00Z",
        Metadata {
            headers: Some(&[
                ("anettokens".into(), "fixture-token".into()),
                ("Accept".into(), "application/json".into()),
            ]),
            content_type: Some("application/json"),
            ..Metadata::default()
        },
    )?;
    let prefix = directory.path().join("039a9fa3b131f0e76da67acba0ecdfde");
    let actual: serde_json::Value =
        serde_json::from_slice(&std::fs::read(prefix.with_extension("meta.json"))?)?;
    anyhow::ensure!(
        actual
            == json!({
                "url": "https://example.test/capture", "method": "GET", "status": 200,
                "representation": [["accept", "application/json"], ["anettokens", "fixture-token"]],
                "content_digest": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
                "bytes": 3, "fetched_at": "2026-09-22T12:00:00Z", "content_type": "application/json",
            })
    );
    anyhow::ensure!(std::fs::read(prefix.with_extension("body"))? == b"abc");
    Ok(())
}
