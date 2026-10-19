use super::*;
use crate::restate_services::wire::{JurisdictionOpen, SourceObjectInventory, SourceObjectOpen};
use census_crawl::CollectionDisposition;
use census_domain::UsJurisdiction;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn inventory(rows: u32) -> OpenWorkReply {
    OpenWorkReply {
        season: "2026-27".into(),
        revision: 1,
        jurisdiction_sweeps: Some(1),
        source_objects: Some(u64::from(rows)),
        source_object_inventory: Some(SourceObjectInventory {
            objects: u64::from(rows),
            accepted_observations: 0,
            accepted_nothing: 0,
            unreadable: 0,
        }),
        silent_sources: Vec::new(),
        jurisdictions: vec![JurisdictionOpen {
            jurisdiction: UsJurisdiction::Michigan,
            identity: "jurisdiction:MI:2026-27:1".into(),
            stages: crate::census::JurisdictionStages::default(),
            unreadable: false,
            teams_stage: None,
            source_refusals: None,
            athleticlive_manifest: None,
            athleticlive_manifest_reason: None,
        }],
        endpoints: (0..rows)
            .map(|index| SourceObjectOpen {
                endpoint: format!("controlled-source:{index}"),
                observations: 0,
                windows: 0,
                unreadable: false,
                disposition: CollectionDisposition::Unknown,
                resolution: None,
                teams_source: None,
            })
            .collect(),
        archive_capture_activity: Default::default(),
        endpoint_page: None,
    }
}

fn measured(
    reply: OpenWorkReply,
    request: Option<&SourceObjectPageRequest>,
) -> Result<OpenWorkReply, String> {
    bounded_with_bytes(reply, request, MAX_RESPONSE_BYTES).map_err(|error| error.to_string())
}

#[test]
fn bounded_pages_cover_every_obligation_without_changing_global_owed_counts() -> TestResult {
    let original = inventory(5);
    let expected = original
        .endpoints
        .iter()
        .map(|row| row.endpoint.clone())
        .collect::<Vec<_>>();
    let mut cursor = None;
    let mut visited = Vec::new();
    for _ in 0..5 {
        let request = SourceObjectPageRequest {
            after: cursor.take(),
            limit: Some(2),
        };
        let page = measured(original.clone(), Some(&request))?;
        let metadata = page.endpoint_page.as_ref().ok_or("page metadata")?;
        check!(eq; metadata.total, 5);
        check!(eq; metadata.offset, u64::try_from(visited.len())?);
        check!(eq; metadata.returned, u64::try_from(page.endpoints.len())?);
        check!(eq; page.source_objects, Some(5));
        check!(eq; page.jurisdiction_sweeps, Some(1));
        check!(eq; page.source_object_inventory.as_ref().ok_or("global inventory")?.objects, 5);
        check!(eq; page.jurisdictions.first().ok_or("jurisdiction")?.identity, "jurisdiction:MI:2026-27:1");
        visited.extend(page.endpoints.into_iter().map(|row| row.endpoint));
        cursor = metadata.next.clone();
        if cursor.is_none() {
            break;
        }
    }
    check!(eq; visited, expected);
    check!(cursor.is_none());
    Ok(())
}

#[test]
fn default_query_bounds_a_native_oversize_inventory_and_exposes_continuation() -> TestResult {
    let mut original = inventory(120_000);
    let padding = "x".repeat(256);
    for row in &mut original.endpoints {
        row.endpoint.push_str(&padding);
    }
    let original_bytes =
        count_response_bytes(&original, u64::MAX).map_err(|error| error.to_string())?;
    check!(original_bytes > 33_816_576);
    let page = measured(original, None)?;
    let metadata = page.endpoint_page.as_ref().ok_or("default page metadata")?;
    check!(eq; metadata.total, 120_000);
    check!(eq; metadata.offset, 0);
    check!(metadata.returned < metadata.total);
    check!(metadata.next.is_some());
    check!(eq; page.source_objects, Some(120_000));
    check!(
        count_response_bytes(&page, u64::MAX).map_err(|error| error.to_string())?
            <= MAX_RESPONSE_BYTES
    );
    Ok(())
}

#[test]
fn cursor_cannot_cross_run_content_or_order_and_rejects_out_of_range_offsets() -> TestResult {
    let original = inventory(3);
    let first = measured(
        original.clone(),
        Some(&SourceObjectPageRequest {
            after: None,
            limit: Some(1),
        }),
    )?;
    let cursor = first
        .endpoint_page
        .ok_or("first page metadata")?
        .next
        .ok_or("continuation")?;
    let request = SourceObjectPageRequest {
        after: Some(cursor),
        limit: Some(1),
    };
    let valid = measured(original.clone(), Some(&request))?;
    check!(eq; valid.endpoints.first().ok_or("next endpoint")?.endpoint, "controlled-source:1");
    for change in 0..4 {
        let mut changed = original.clone();
        match change {
            0 => changed.season = "2025-26".into(),
            1 => changed.revision = 2,
            2 => {
                changed
                    .endpoints
                    .first_mut()
                    .ok_or("changed endpoint")?
                    .observations = 1
            }
            _ => changed.endpoints.reverse(),
        }
        let error = bounded_with_bytes(changed, Some(&request), MAX_RESPONSE_BYTES)
            .err()
            .ok_or("changed inventory admitted")?;
        check!(eq; error.code(), 400);
    }
    let digest = serialized_digest(&(
        original.season.as_str(),
        original.revision,
        &original.endpoints,
    ))?;
    let beyond = SourceObjectPageRequest {
        after: Some(format!("{digest}:4")),
        limit: Some(1),
    };
    let error = bounded_with_bytes(original.clone(), Some(&beyond), MAX_RESPONSE_BYTES)
        .err()
        .ok_or("offset beyond inventory admitted")?;
    check!(eq; error.code(), 400);
    for cursor in ["", "no-separator", "wrong-digest:0"] {
        let malformed = SourceObjectPageRequest {
            after: Some(cursor.into()),
            limit: Some(1),
        };
        let error = bounded_with_bytes(original.clone(), Some(&malformed), MAX_RESPONSE_BYTES)
            .err()
            .ok_or("malformed cursor admitted")?;
        check!(eq; error.code(), 400);
    }
    for limit in [
        0,
        u16::try_from(MAX_PAGE)?
            .checked_add(1)
            .ok_or("limit overflow")?,
    ] {
        let request = SourceObjectPageRequest {
            after: None,
            limit: Some(limit),
        };
        let error = bounded_with_bytes(original.clone(), Some(&request), MAX_RESPONSE_BYTES)
            .err()
            .ok_or("invalid limit admitted")?;
        check!(eq; error.code(), 400);
    }
    Ok(())
}

#[test]
fn full_encoded_reply_accepts_exact_bytes_and_refuses_one_byte_short() -> TestResult {
    let original = inventory(3);
    let request = SourceObjectPageRequest {
        after: None,
        limit: Some(2),
    };
    let expected = measured(original.clone(), Some(&request))?;
    let exact = count_response_bytes(&expected, u64::MAX).map_err(|error| error.to_string())?;
    let accepted = bounded_with_bytes(original.clone(), Some(&request), exact)
        .map_err(|error| error.to_string())?;
    check!(eq; accepted.endpoint_page, expected.endpoint_page);
    check!(eq; accepted.endpoints.first().ok_or("first endpoint")?.endpoint, "controlled-source:0");
    let short = exact.checked_sub(1).ok_or("encoded boundary")?;
    let error = bounded_with_bytes(original, Some(&request), short)
        .err()
        .ok_or("short byte grant admitted")?;
    check!(eq; error.code(), 413);
    let mut oversized_fixed = inventory(1);
    oversized_fixed
        .silent_sources
        .push("x".repeat(usize::try_from(MAX_RESPONSE_BYTES)?));
    let error = bounded_with_bytes(oversized_fixed, Some(&request), MAX_RESPONSE_BYTES)
        .err()
        .ok_or("oversized fixed metadata admitted")?;
    check!(eq; error.code(), 413);
    Ok(())
}

#[test]
fn empty_inventory_is_measured_zero_without_an_invented_continuation() -> TestResult {
    let page = measured(inventory(0), None)?;
    let metadata = page.endpoint_page.ok_or("empty page metadata")?;
    check!(eq; (metadata.total, metadata.offset, metadata.returned), (0, 0, 0));
    check!(eq; page.source_objects, Some(0));
    check!(page.endpoints.is_empty());
    check!(metadata.next.is_none());
    Ok(())
}
