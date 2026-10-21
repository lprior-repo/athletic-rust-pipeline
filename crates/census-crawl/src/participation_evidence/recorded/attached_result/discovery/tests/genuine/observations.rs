use super::*;
use census_domain::model::{ExactSeconds, Mark};

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Observed {
    document: Vec<u8>,
    rows: Vec<Vec<u8>>,
}

pub(super) fn observe(
    document: DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
) -> TestResult<Observed> {
    check!(eq; document.row_count(), 3);
    check!(document.row(3).is_none());
    let checked = document
        .checked_document()
        .as_milesplit()
        .ok_or("genuine MileSplit document")?;
    check!(eq; checked.context_reference(), document.recorded_root().key);
    check!(eq; checked.meet_id(), "763888");
    check!(eq; checked.result_document_id(), "1304896");
    let page = checked.parsed_page();
    let wire = serde_json::to_vec(&(
        document.recorded_root(),
        checked.context_reference(),
        checked.original_document(),
        (
            &page.meet,
            &page.sport,
            &page.region,
            &page.school_year,
            &page.skipped,
            &page.grade_issues,
        ),
        checked.published_start(),
        checked.published_end(),
    ))?;
    let mut rows = Vec::new();
    for (index, (ordinal, offset, length, mark, wind)) in [
        (527, 80520, 100, "13.57", Some(0.2)),
        (540, 81648, 92, "14.10", None),
        (572, 84465, 92, "36.23", None),
    ]
    .into_iter()
    .enumerate()
    {
        let linked = document.row(index).ok_or("discovered genuine row absent")?;
        let row = linked
            .checked_row()
            .as_milesplit()
            .ok_or("genuine MileSplit row")?;
        let attachment = linked.attachment();
        check!(eq; row.locator(), SourceRowLocator { ordinal, byte_offset: offset, byte_length: length });
        check!(eq; row.parsed_row().mark, Mark::TimeSeconds(ExactSeconds::parse(mark)?));
        check!(eq; row.parsed_row().wind_mps, wind);
        check!(eq; attachment.subject().interpretation.as_str(), row.reference());
        check!(eq; attachment.subject().subject.as_str(), row.subject()?.reference());
        check!(eq; attachment.subject().capture, document.recorded_root().payload.digest);
        check!(eq; attachment.subject().context.as_str(), checked.context_reference());
        check!(eq; attachment.inventory().records, 2);
        check!(eq; attachment.record().command.operation.as_str(), "genuine-reviewed-may15-b-v1");
        check!(eq; attachment.receipt().operation, "evidence_subject_attachment_v1:genuine-reviewed-may15-b-v1");
        check!(eq; linked.gaps(), &[crate::participation_evidence::recorded::AttachedResultGapV1::SchoolBindingUnresolved]);
        rows.push(serde_json::to_vec(&(
            row.locator(),
            row.reference(),
            row.original_row()?,
            row.original_row_digest(),
            attachment.subject(),
            attachment.target(),
            attachment.record(),
            attachment.receipt(),
            attachment.inventory(),
            attachment.store_identity(),
            attachment.accepted_public_head(),
            attachment.snapshot_sequence(),
        ))?);
    }
    Ok(Observed {
        document: wire,
        rows,
    })
}
