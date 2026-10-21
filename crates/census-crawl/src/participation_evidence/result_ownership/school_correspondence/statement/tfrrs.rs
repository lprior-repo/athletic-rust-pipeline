use super::{domain, invalid, number, ownership, reference, Objects};
use crate::participation_evidence::result_ownership::tfrrs::{
    team_text::read_original_team_text, CheckedRetainedTfrrsListV1, CheckedRetainedTfrrsRowV1,
};
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::{AttachmentReplayGrantV1, AttachmentTextV1};
use census_domain::result_school_correspondence::{
    ResultSchoolSourceParserV1, ResultSchoolStatementClaimV1,
};
use census_store::{
    RetainedCaptureLeafKindV1, RetainedCaptureLeafV1, RetainedCaptureRootV1, RetainedCaptureSlotV1,
};

pub(super) fn build(
    root: &RetainedCaptureRootV1,
    document: &CheckedRetainedTfrrsListV1<'_>,
    row: &CheckedRetainedTfrrsRowV1<'_>,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<ResultSchoolStatementClaimV1> {
    let quote = read_original_team_text(row)
        .map_err(|_| invalid("original unique untransformed TFRRS team text unavailable"))?;
    let body = reference(document.original_document().as_bytes(), grant)?;
    if body.digest.as_str() != document.capture().body_sha256() {
        return Err(invalid("TFRRS school original body digest differs"));
    }
    objects.retain(RetainedCaptureLeafV1 {
        version: 1,
        root: root.clone(),
        slot: RetainedCaptureSlotV1::SelfCapture,
        kind: RetainedCaptureLeafKindV1::OriginalBody,
        object: body.clone(),
    })?;
    let parser = ResultSchoolSourceParserV1::Tfrrs;
    let interpretation = objects.json(
        &(
            parser.row_interpretation(),
            document.capture().requested_url(),
            document.capture().body_sha256(),
            row.locator(),
        ),
        grant,
    )?;
    if interpretation.digest.as_str() != row.reference() {
        return Err(invalid("TFRRS school row interpretation differs"));
    }
    let subject = row.subject().map_err(ownership)?;
    let locator = reference(subject.locator().as_bytes(), grant)?;
    let span = quote.span();
    let length = span
        .end
        .checked_sub(span.start)
        .ok_or(census_store::StoreError::CounterOverflow)?;
    Ok(ResultSchoolStatementClaimV1 {
        result_root: objects.json(root, grant)?,
        body,
        interpretation,
        row_locator: locator.digest,
        row_digest: reference(row.original_row().map_err(ownership)?.as_bytes(), grant)?.digest,
        row_offset: number(row.locator().byte_offset)?,
        row_length: number(row.locator().byte_length)?,
        statement_offset: number(span.start)?,
        statement_length: number(length)?,
        statement_digest: reference(quote.text().as_bytes(), grant)?.digest,
        statement_text: AttachmentTextV1::parse(quote.text()).map_err(domain)?,
        parser_contract: objects.bytes(parser.contract(), grant)?,
    })
}
