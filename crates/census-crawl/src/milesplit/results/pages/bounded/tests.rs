use super::super::tests::{options, scratch, seed_cache};
use super::*;
use census_domain::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const URL: &str = "https://oh.milesplit.com/meets/123-fixture/results";

fn page() -> MeetPage {
    MeetPage {
        results_url: URL.to_string(),
        jurisdiction: UsJurisdiction::Ohio,
    }
}

#[tokio::test]
async fn bounded_discovery_preserves_literal_legacy_and_inline_url_rules() -> TestResult {
    for html in [
        "let meetResultFiles = [{\"id\":4,\"name\":\"Results\",\"isMeetPro\":0}];",
        "<select id=\"ddResultsPage\"><option value=\"https://oh.milesplit.com/meets/123-fixture/results/4\">Results</option></select>",
        "<div id=\"meetResultsBody\">Results</div>",
    ] {
        let (_dir, fetcher) = scratch()?;
        seed_cache(fetcher.cache_dir(), URL, 200, html)?;
        let historical = super::super::read_meet_pages(&fetcher, [page()], &options()).await?;
        let envelope = read_meet_pages_bounded(&fetcher, [page()], &options(), MeetPageLimits::new(64 * 1024 * 1024)).await;
        let (bounded, requested) = envelope.into_parts();
        let bounded = bounded?;
        check!(eq; bounded.files, historical.files);
        check!(eq; bounded.pages_read, historical.pages_read);
        check!(requested > 0);
    }
    Ok(())
}

#[tokio::test]
async fn bounded_discovery_file_cap_and_allocation_refusal_preserve_spent_usage() -> TestResult {
    let (_dir, fetcher) = scratch()?;
    let files = (0..4097)
        .map(|id| format!("{{\"id\":{id},\"name\":\"Results\"}}"))
        .collect::<Vec<_>>()
        .join(",");
    seed_cache(
        fetcher.cache_dir(),
        URL,
        200,
        &format!("let meetResultFiles = [{files}];"),
    )?;
    let (outcome, requested) = read_meet_pages_bounded(
        &fetcher,
        [page()],
        &options(),
        MeetPageLimits::new(64 * 1024 * 1024),
    )
    .await
    .into_parts();
    check!(matches!(
        outcome,
        Err(CrawlError::Resource {
            resource: "meet-page files",
            requested: 4097,
            limit: 4096
        })
    ));
    check!(requested > 0);
    let (outcome, spent) =
        read_meet_pages_bounded(&fetcher, [page()], &options(), MeetPageLimits::new(8192))
            .await
            .into_parts();
    check!(matches!(
        outcome,
        Err(CrawlError::Resource {
            resource: "meet-page allocation",
            ..
        })
    ));
    check!(eq; spent, 8192);
    Ok(())
}

#[tokio::test]
async fn bounded_discovery_schema_quarantine_retains_usage_and_stops_at_existing_limit(
) -> TestResult {
    let (_dir, fetcher) = scratch()?;
    seed_cache(fetcher.cache_dir(), URL, 200, "<h1>foreign page</h1>")?;
    let (outcome, requested) = read_meet_pages_bounded(
        &fetcher,
        (0..26).map(|_| page()),
        &options(),
        MeetPageLimits::new(64 * 1024 * 1024),
    )
    .await
    .into_parts();
    let outcome = outcome?;
    check!(outcome.stopped());
    check!(eq; outcome.quarantined.len(), super::super::MISMATCH_LIMIT);
    check!(eq; outcome.pages_read, 0);
    check!(requested > 0);
    Ok(())
}

#[test]
fn bounded_raw_urls_accept_exact_cap_before_growth_and_match_existing_encoder() -> TestResult {
    let file = crate::milesplit::MeetResultFile {
        id: i64::MIN,
        name: String::new(),
        is_meet_pro: 0,
        inline: false,
    };
    let base = "a".repeat(4096 - 25);
    let mut allocation = Allocation {
        limit: 8192,
        requested: 0,
    };
    let actual = raw_url(&file, &base, &mut allocation)?;
    check!(eq; actual.len(), 4096);
    check!(eq; actual, file.raw_url(&base));
    let before = allocation.requested;
    check!(raw_url(&file, &format!("{base}a"), &mut allocation).is_err());
    check!(eq; allocation.requested, before);
    Ok(())
}

#[test]
fn admitted_single_buffer_lossy_decode_matches_original_byte_replacement_semantics() -> TestResult {
    for bytes in [
        b"\xffa\xfe".as_slice(),
        b"a\xe2\x82".as_slice(),
        b"\xf0\x9f\x92\xa9\xff".as_slice(),
        b"\xc0\xaf\xed\xa0\x80".as_slice(),
    ] {
        let capacity = bytes.len().checked_mul(3).ok_or("capacity")?;
        let mut allocation = Allocation {
            limit: u64::try_from(capacity)?,
            requested: 0,
        };
        let actual = lossy_body(bytes, &mut allocation)?;
        check!(eq; actual, String::from_utf8_lossy(bytes));
        check!(eq; allocation.requested, u64::try_from(capacity)?);
        let mut below = Allocation {
            limit: u64::try_from(capacity.checked_sub(1).ok_or("capacity")?)?,
            requested: 0,
        };
        check!(lossy_body(bytes, &mut below).is_err());
        check!(eq; below.requested, 0);
    }
    Ok(())
}
