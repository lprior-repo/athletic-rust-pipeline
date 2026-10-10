use super::*;
use crate::athleticlive::wire::standings_url;
use crate::net::cache::{content_digest, CacheMeta};
use std::collections::BTreeMap;

const TEST_CAPTURED_AT: &str = "2026-09-22T12:00:00Z";

pub(super) async fn collect(
    ctx: &AdapterContext<'_>,
    options: &ResultOptions,
) -> TestResult<AdapterReport> {
    let mut qualified = options.clone();
    let target = qualified.meet.as_ref().ok_or("fixture meet target")?;
    qualified
        .capture_metadata
        .extend(documents(&qualified.documents)?);
    if let Some(path) = qualified.summary.as_ref() {
        let body = std::fs::read(path)?;
        qualified.capture_metadata.insert(
            path.clone(),
            metadata(
                event_summary_url(target.athleticlive_meet_id),
                &body,
                TEST_CAPTURED_AT,
            ),
        );
    }
    qualified
        .standings
        .iter()
        .try_for_each(|standing| -> TestResult {
            let url = standings_url(target.athleticlive_meet_id, &standing.run_id)
                .ok_or("fixture standing URL")?;
            let body = std::fs::read(&standing.path)?;
            qualified.capture_metadata.insert(
                standing.path.clone(),
                metadata(url, &body, TEST_CAPTURED_AT),
            );
            Ok(())
        })?;
    Ok(super::super::collect(ctx, &qualified).await?)
}

pub(super) fn documents(paths: &[String]) -> TestResult<BTreeMap<String, CacheMeta>> {
    paths
        .iter()
        .map(|path| {
            let body = std::fs::read(path)?;
            let text = std::str::from_utf8(&body)?;
            let document: serde_json::Value = serde_json::from_str(text)?;
            let id = document
                .pointer("/_source/i")
                .and_then(crate::athleticlive::docs::value_u64)
                .ok_or("fixture native event ID")?;
            let acquired_at = if text == XC_STATE {
                "2026-09-22T03:59:19Z"
            } else if text == HJ_MITS {
                "2026-09-22T04:01:13Z"
            } else {
                TEST_CAPTURED_AT
            };
            Ok((
                path.clone(),
                metadata(event_doc_url(id), &body, acquired_at),
            ))
        })
        .collect()
}

fn metadata(url: String, body: &[u8], acquired_at: &str) -> CacheMeta {
    CacheMeta {
        redirects: Vec::new(),
        url,
        response_url: None,
        method: "GET".into(),
        status: 200,
        representation: crate::net::RepresentationHeaders::default(),
        content_digest: content_digest(body),
        bytes: body.len(),
        fetched_at: acquired_at.into(),
        etag: None,
        last_modified: None,
        content_type: Some("application/json".into()),
    }
}
