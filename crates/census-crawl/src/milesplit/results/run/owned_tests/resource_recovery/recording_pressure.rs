use super::*;
use crate::milesplit::{OwnedCaptureRetention, OwnedResourceReceipt};

#[test]
fn acquired_recording_refusal_keeps_exact_resource_fields_and_report_only_body_locator(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let (_dir, store, fetcher, reference) = setup()?;
        seed_owned(&fetcher, &reference, TROY)?;
        seed_metadata(&fetcher, &reference)?;
        let recording = crate::Recording::new();
        let mut pressure = crate::recording::RowSink::Record { store: &store, recording: &recording }.write_batch();
        let bytes = crate::recording::MAX_RECORDED_BYTES.checked_sub(2100).ok_or("pressure budget")?;
        pressure.journal_done("capacity_pressure", "reserved", &"x".repeat(bytes))?;
        pressure.commit()?;
        let held = recording.usage();
        let expected = held.retained_bytes.checked_add(32 * 1024 * 4).ok_or("capture admission accounting")?;
        let mut ctx = context(&store, &fetcher)?;
        ctx.recording = Some(&recording);
        let archived = match crate::milesplit::owned::read_owned_meet(&ctx, &reference).await {
            Err(CrawlError::OwnedCaptureResource(failure)) => {
                check!(eq; failure.resource(), "recorded retained bytes");
                check!(eq; failure.requested(), expected);
                check!(eq; failure.limit(), crate::recording::MAX_RECORDED_BYTES);
                match failure.retention() {
                    OwnedCaptureRetention::Captured { capture, immutable_body, receipt } => {
                        check!(eq; *receipt, OwnedResourceReceipt::ReportOnly);
                        check!(eq; capture.content_digest, crate::net::cache::content_digest(TROY));
                        check!(eq; capture.body.len(), 0);
                        check!(eq; std::fs::read(immutable_body)?, TROY);
                        immutable_body.clone()
                    }
                    other => return Err(format!("acquired recording refusal lost its witness: {other:?}").into()),
                }
            }
            outcome => return Err(format!("recording refusal must preserve a resource carrier: {outcome:?}").into()),
        };
        let second = ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266815/raw").ok_or("later locator")?;
        let mut supplied = options(&reference);
        supplied.urls.extend(options(&second).urls);
        let report = crate::milesplit::collect_result_sets(&ctx, &supplied).await?;
        check!(eq; (report.rows, report.from_cache), (0, 1));
        check!(eq; report.disposition, crate::CollectionDisposition::Partial);
        check!(eq; report.unfinished, supplied.urls.iter().map(|request| request.url.clone()).collect::<Vec<_>>());
        let digest = crate::net::cache::content_digest(TROY);
        check!(report.notes.iter().any(|note| note.contains(&format!("owned capture {digest}"))
            && note.contains(&archived.display().to_string()) && note.contains("report-only")));
        check!(!report.notes.iter().any(|note| note.contains("owned capture unacquired")));
        check!(eq; recording.usage(), held);
        no_canonical_effects(&store)?;
        let staged = recording.drain();
        check!(eq; staged.rows.len(), 0);
        check!(eq; staged.journal.len(), 1);
        check!(eq; staged.journal.first().ok_or("retained pressure")?.phase, "capacity_pressure");
        check!(eq; std::fs::read(archived)?, TROY);
        Ok(())
    })
}
