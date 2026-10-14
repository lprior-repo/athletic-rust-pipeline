use super::*;
use crate::milesplit::{
    OwnedCaptureResourceFailure, OwnedCaptureRetention, OwnedParseArithmetic, OwnedParseLocator,
    OwnedParseResource, OwnedParseResourceFailure, OwnedResourceReceipt,
};

fn parse_origins() -> TestResult<Vec<(OwnedParseResourceFailure, String)>> {
    let source = match Vec::<u8>::new().try_reserve(usize::MAX) {
        Err(source) => source,
        Ok(()) => return Err("oversized reservation unexpectedly succeeded".into()),
    };
    check!(eq; source.to_string(), "memory allocation failed because the computed capacity exceeded the collection's maximum");
    Ok(vec![
        (
            OwnedParseResourceFailure::Limit {
                resource: OwnedParseResource::ArrayValues,
                locator: OwnedParseLocator::Byte { offset: 37 },
                requested: 20_001,
                limit: 20_000,
                retention: OwnedCaptureRetention::Unrecorded,
            },
            "owned array values reservation of 20001 exceeds limit 20000 at byte[37]".into(),
        ),
        (
            OwnedParseResourceFailure::Arithmetic {
                resource: OwnedParseResource::DecodeWork,
                locator: OwnedParseLocator::Byte { offset: 19 },
                operation: OwnedParseArithmetic::TokenIncrement,
                retention: OwnedCaptureRetention::Unrecorded,
            },
            "arithmetic overflow: owned decode admission overflow (owned decode work/TokenIncrement) at byte[19]".into(),
        ),
        (
            OwnedParseResourceFailure::Allocation {
                resource: OwnedParseResource::AcceptedRows,
                locator: OwnedParseLocator::Data,
                requested: usize::MAX,
                source,
                retention: OwnedCaptureRetention::Unrecorded,
            },
            format!("owned accepted rows reservation of {} failed: memory allocation failed because the computed capacity exceeded the collection's maximum at data", usize::MAX),
        ),
    ])
}

fn assert_origin_root(
    ctx: &AdapterContext<'_>,
    reference: &ResultSetRef,
    error: CrawlError,
    origin: &str,
    receipt: OwnedResourceReceipt,
) -> TestResult {
    let captured = crate::milesplit::results::run::resource::resource_capture(&error)
        .ok_or("retained capture")?;
    let path = ctx
        .fetcher
        .cache_dir()
        .join("archive")
        .join("bodies")
        .join(format!("{}.body", captured.content_digest));
    check!(eq; std::fs::read(&path)?, TROY);
    let expected = json!({
        "source_url": reference.url,
        "owned_capture": crate::milesplit::results::run::capture::provenance(captured),
        "raw_metadata_capture": null,
        "performance_as_of": ctx.performance_as_of,
        "disposition": "resource_limit", "unfinished": true, "resource_error": origin,
    });
    let key = format!(
        "partial/resource-v2/{}",
        census_domain::model::serialized_digest(&expected)?
    );
    let detail = error.to_string();
    check!(detail.starts_with(origin));
    check!(detail.contains(&path.display().to_string()));
    check!(eq; detail.contains("report-only"), receipt == OwnedResourceReceipt::ReportOnly);
    let mut run = Run::new(ProviderSchools::from_schools(&[]));
    run.resource(ctx, &reference.url, &error)?;
    check!(eq; ctx.store.journal_payload(crate::milesplit::RESULT_SET_PHASE, &key)?, Some(expected));
    check!(matches!(run.intake, Intake::Blocked { .. }));
    check!(matches!(run.stats.resource_stop,
        crate::milesplit::results::ResourceStop::Unfinished(ref locator) if locator.contains(&detail)));
    Ok(())
}

#[test]
fn typed_origin_roots_preserve_every_cause_and_ignore_all_retention_states() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let (_dir, store, fetcher, reference) = setup()?;
        seed_owned(&fetcher, &reference, TROY)?;
        let ctx = context(&store, &fetcher)?;
        let mut retained = None;
        for receipt in [OwnedResourceReceipt::NotRequested, OwnedResourceReceipt::Retained, OwnedResourceReceipt::ReportOnly] {
            for (failure, origin) in parse_origins()? {
                check!(eq; failure.stable_origin().to_string(), origin);
                check!(eq; failure.to_string(), origin);
                let capture = crate::milesplit::fetch::fetch_owned_capture(&ctx, &reference, &ctx.fetch_options()).await?.capture;
                let path = fetcher.cache_dir().join("archive").join("bodies")
                    .join(format!("{}.body", capture.content_digest));
                let failure = failure.retain(capture, path, receipt);
                check!(eq; failure.stable_origin().to_string(), origin);
                assert_origin_root(&ctx, &reference, failure.into(), &origin, receipt)?;
            }
            let capture = crate::milesplit::fetch::fetch_owned_capture(&ctx, &reference, &ctx.fetch_options()).await?.capture;
            let path = fetcher.cache_dir().join("archive").join("bodies")
                .join(format!("{}.body", capture.content_digest));
            let failure = OwnedCaptureResourceFailure::new("recorded retained bytes", 101, 100, capture, path, receipt);
            let origin = "recorded retained bytes reservation of 101 exceeds limit 100";
            check!(eq; failure.stable_origin().to_string(), origin);
            check!(eq; (failure.resource(), failure.requested(), failure.limit()), ("recorded retained bytes", 101, 100));
            assert_origin_root(&ctx, &reference, failure.into(), origin, receipt)?;
            let snapshot = journal_snapshot(&store, crate::milesplit::RESULT_SET_PHASE)?;
            check!(eq; snapshot.len(), 4);
            match &retained {
                Some(prior) => check!(eq; &snapshot, prior),
                None => retained = Some(snapshot),
            }
        }
        no_canonical_effects(&store)?;
        Ok(())
    })
}

#[test]
fn plain_generic_resource_keeps_its_exact_legacy_payload_and_root() -> TestResult {
    let (_dir, store, fetcher, reference) = setup()?;
    let ctx = context(&store, &fetcher)?;
    let error = CrawlError::Resource {
        resource: "raw metadata lines",
        requested: 20_001,
        limit: 20_000,
    };
    let expected = json!({
        "source_url": reference.url, "owned_capture": null, "raw_metadata_capture": null,
        "performance_as_of": ctx.performance_as_of,
        "disposition": "resource_limit", "unfinished": true,
        "resource_error": "raw metadata lines reservation of 20001 exceeds limit 20000",
    });
    let key = format!(
        "partial/resource/{}",
        census_domain::model::serialized_digest(&expected)?
    );
    let mut run = Run::new(ProviderSchools::from_schools(&[]));
    run.resource(&ctx, &reference.url, &error)?;
    check!(eq; journal_snapshot(&store, crate::milesplit::RESULT_SET_PHASE)?, std::collections::BTreeMap::from([(key, expected)]));
    no_canonical_effects(&store)?;
    Ok(())
}
