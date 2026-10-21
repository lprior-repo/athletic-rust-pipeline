use super::{domain, invalid, objects::Objects, ownership, reference};
use crate::participation_evidence::result_ownership::{
    CheckedRetainedResultDocumentV1, CheckedRetainedResultRowV1, ResultSchoolStatementV1,
};
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentReplayGrantV1, AttachmentTextV1,
};
use census_domain::result_school_correspondence::{
    ResultSchoolStatementClaimV1, RESULT_SCHOOL_PARSER_CONTRACT_V1,
};
use census_store::{
    RetainedCaptureLeafKindV1, RetainedCaptureLeafV1, RetainedCaptureRootV1, RetainedCaptureSlotV1,
};

mod tfrrs;

pub(super) fn build_source(
    root: &RetainedCaptureRootV1,
    document: &super::super::CheckedHtmlResultDocumentV1<'_>,
    row: &super::super::CheckedHtmlResultRowV1<'_>,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<ResultSchoolStatementClaimV1> {
    use super::super::{CheckedHtmlResultDocumentV1 as Document, CheckedHtmlResultRowV1 as Row};
    match (document, row) {
        (Document::MileSplit(document), Row::MileSplit(row)) => {
            build(root, document, row, objects, grant)
        }
        (Document::Tfrrs(document), Row::Tfrrs(row)) => {
            tfrrs::build(root, document, row, objects, grant)
        }
        _ => Err(invalid("school statement and original source differ")),
    }
}

pub(super) fn build(
    root: &RetainedCaptureRootV1,
    document: &CheckedRetainedResultDocumentV1<'_>,
    row: &CheckedRetainedResultRowV1<'_>,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<ResultSchoolStatementClaimV1> {
    let quote = match row.school_statement().map_err(ownership)? {
        ResultSchoolStatementV1::Available(quote) => quote,
        ResultSchoolStatementV1::Unavailable => {
            return Err(invalid("original unique school cell unavailable"))
        }
    };
    let body = reference(document.original_document().as_bytes(), grant)?;
    if body.digest.as_str() != document.capture().body_sha256() {
        return Err(invalid("school original body digest differs"));
    }
    objects.retain(RetainedCaptureLeafV1 {
        version: 1,
        root: root.clone(),
        slot: RetainedCaptureSlotV1::SelfCapture,
        kind: RetainedCaptureLeafKindV1::OriginalBody,
        object: body.clone(),
    })?;
    let interpretation = objects.json(
        &(
            "retained-milesplit-html-result-row/v1",
            document.capture().requested_url(),
            document.capture().body_sha256(),
            row.locator(),
        ),
        grant,
    )?;
    if interpretation.digest.as_str() != row.reference() {
        return Err(invalid("school original row interpretation differs"));
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
        row_digest: AttachmentDigestV1::parse(quote.row_digest()).map_err(domain)?,
        row_offset: number(quote.row().byte_offset)?,
        row_length: number(quote.row().byte_length)?,
        statement_offset: number(span.start)?,
        statement_length: number(length)?,
        statement_digest: AttachmentDigestV1::parse(quote.digest()).map_err(domain)?,
        statement_text: AttachmentTextV1::parse(quote.text()).map_err(domain)?,
        parser_contract: objects.bytes(RESULT_SCHOOL_PARSER_CONTRACT_V1, grant)?,
    })
}

fn number(value: usize) -> CrawlResult<u64> {
    u64::try_from(value).map_err(|_| census_store::StoreError::CounterOverflow.into())
}
