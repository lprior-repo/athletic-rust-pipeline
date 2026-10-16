use super::provider_target::SourceTargetPacketV1;
use super::result_ownership::CheckedRetainedResultDocumentV1;
use super::{IndependentParticipationPacketV1, MAX_PACKET};
use crate::recording::RowSink;
use crate::{CrawlResult, Recording};
use census_store::{ReadBudget, RetainedCaptureRootSchemaV1, Store};

mod attached_result;
mod provider;
mod provider_school;
mod read;
mod schools;

pub use attached_result::{
    discover_current_html_results, discover_frozen_html_results, with_attached_html_results,
    AttachedResultGapV1, AttachmentHtmlResultDiscoveryGapV1, AttachmentHtmlResultDiscoveryV1,
    DiscoveredIdentityLinkedHtmlResultDocumentV1, DiscoveredIdentityLinkedHtmlResultRowV1,
    DiscoveredResultAttachmentV1, IdentityLinkedHtmlResultDocumentV1,
    IdentityLinkedHtmlResultRowV1, UnresolvedAttachmentHtmlResultV1,
};
pub use provider::{ProviderTargetIntakeDispositionV1, ProviderTargetWithheldV1};
pub use provider_school::{
    read_recorded_provider_school, record_provider_school_intake, RecordedProviderSchoolV1,
};
pub use read::{
    read_recorded_html_result_capture, read_recorded_independent_participation,
    read_recorded_provider_target, RecordedHtmlResultCaptureV1, RecordedIndependentParticipationV1,
    RecordedProviderTargetV1,
};

pub fn record_provider_target_intake(
    store: &Store,
    recording: &Recording,
    packet: &SourceTargetPacketV1,
    school_budget: ReadBudget,
) -> CrawlResult<ProviderTargetIntakeDispositionV1> {
    let checked = packet.read()?;
    let snapshot = store.snapshot();
    let schools = schools::read(&snapshot, school_budget)?;
    let projected = provider::project(&checked, &schools)?;
    super::codec::encode(packet, MAX_PACKET)?;
    let payload = serde_json::to_value(packet).map_err(super::json)?;
    let schema = RetainedCaptureRootSchemaV1::ProviderTargetPacketV1;
    Store::check_retained_capture_entry_v1(schema, checked.reference().as_str(), &payload)?;
    let mut batch = RowSink::Record { store, recording }.write_batch();
    batch.journal_done(schema.phase(), checked.reference().as_str(), &payload)?;
    projected.append(&mut batch)?;
    batch.commit()?;
    Ok(projected.disposition)
}

pub fn record_retained_html_result_capture(
    store: &Store,
    recording: &Recording,
    document: &CheckedRetainedResultDocumentV1<'_>,
) -> CrawlResult<()> {
    super::codec::encode(document.capture(), MAX_PACKET)?;
    let payload = serde_json::to_value(document.capture()).map_err(super::json)?;
    let schema = RetainedCaptureRootSchemaV1::HtmlResultCaptureV1;
    Store::check_retained_capture_entry_v1(schema, document.context_reference(), &payload)?;
    let mut batch = RowSink::Record { store, recording }.write_batch();
    batch.journal_done(schema.phase(), document.context_reference(), &payload)?;
    batch.commit()
}

pub fn record_independent_participation_packet(
    store: &Store,
    recording: &Recording,
    packet: &IndependentParticipationPacketV1,
) -> CrawlResult<()> {
    let snapshot = store.snapshot();
    let checked = packet.read(snapshot.store_identity()?, snapshot.sequence())?;
    let payload = packet.payload()?;
    let schema = RetainedCaptureRootSchemaV1::ParticipationPacketV1;
    Store::check_retained_capture_entry_v1(schema, checked.reference().as_str(), &payload)?;
    let mut batch = RowSink::Record { store, recording }.write_batch();
    batch.journal_done(schema.phase(), checked.reference().as_str(), &payload)?;
    batch.commit()
}

#[cfg(test)]
mod tests;
