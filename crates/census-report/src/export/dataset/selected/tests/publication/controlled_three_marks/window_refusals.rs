use super::source::{self, TestResult};
use super::window_source::{SOURCE_BODY, SOURCE_URL};
use census_crawl::participation_evidence::recorded::record_independent_participation_window_packet;
use census_crawl::participation_evidence::IndependentParticipationWindowPacketV1;
use census_crawl::{CrawlError, Recording};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::{RetainedCaptureRootSchemaV1 as Schema, StoreError};

fn outside_owned_article() -> TestResult<String> {
    let start = SOURCE_BODY
        .find("<p class=\"participation-window\">")
        .ok_or("controlled window paragraph missing")?;
    let before = SOURCE_BODY.get(..start).ok_or("window prefix span")?;
    let rest = SOURCE_BODY.get(start..).ok_or("window suffix span")?;
    let (paragraph, closing) = rest
        .split_once("</p>")
        .ok_or("controlled window paragraph close missing")?;
    let tail = closing
        .strip_prefix("</article>")
        .ok_or("controlled window article close missing")?;
    Ok(format!("{before}</article>{paragraph}</p>{tail}"))
}

fn refuses_without_mutation(body: &str) -> TestResult {
    let installed = source::install()?;
    let store = &installed.store;
    let capture = source::capture(
        SOURCE_URL,
        body.as_bytes(),
        b"HTTP/1.1 200 OK\r\ncontent-type: text/html; charset=utf-8\r\n\r\n",
        "2026-10-16T00:13:00Z",
        "2026-10-16T00:13:01Z",
    )?;
    let packet = IndependentParticipationWindowPacketV1::new(capture)?;
    let reference = packet.reference()?;
    let recording = Recording::new();
    let before = store.snapshot();
    let identity = before.store_identity()?;
    let receipts = store.receipt_count()?;
    let result = record_independent_participation_window_packet(store, &recording, &packet);
    let after = store.snapshot();
    let after_receipts = store.receipt_count()?;
    match result {
        Err(CrawlError::Store(StoreError::Refused { .. })) => {}
        Err(error) => return Err(error.into()),
        Ok(()) => return Err("invalid window was accepted by public record API".into()),
    }
    check!(eq; after.sequence(), before.sequence());
    check!(eq; after.evidence_generation(), before.evidence_generation());
    check!(eq; after.derived_generation(), before.derived_generation());
    check!(eq; after_receipts, receipts);
    check!(eq; after.store_identity()?, identity);
    let schema = Schema::ParticipationWindowPacketV1;
    let mut grant = AttachmentReplayGrantV1::new();
    let typed = after.read_retained_capture_root_v1(schema, reference.as_str(), &mut grant)?;
    check!(eq; typed.map(|held| held.root().clone()), None);
    source::require_originals(store, &installed.source_records)?;
    source::require_identity(store, &installed.candidate, &installed.public)
}

#[test]
fn public_window_record_refuses_semantic_mismatches_without_changing_authority() -> TestResult {
    [
        SOURCE_BODY.replace("cohort: 2027", "cohort: 2028"),
        SOURCE_BODY.replace("school year: 2025-26", "school year: 2024-25"),
        outside_owned_article()?,
    ]
    .into_iter()
    .try_for_each(|body| refuses_without_mutation(&body))
}
