use super::super::{
    codec, hash,
    result_ownership::{CheckedRetainedResultDocumentV1, CheckedRetainedResultRowV1},
};
use super::{
    HtmlResultProjectionError as Error, ProposedHtmlResultTargetV1, Result, INTERPRETATION,
    SOURCE_KEY,
};
use census_domain::model::{Evidence, SourceRef};
use serde::Serialize;

pub(super) fn source_key(
    document: &CheckedRetainedResultDocumentV1<'_>,
    row: &CheckedRetainedResultRowV1<'_>,
) -> Result<String> {
    let tuple = (
        SOURCE_KEY,
        document.meet_id(),
        document.result_document_id(),
        document.capture().requested_url(),
        document.capture().body_sha256(),
        row.locator(),
    );
    let bytes = codec::encode(&tuple, 4096)?;
    let digest = hash(&bytes);
    let mut key = String::new();
    let capacity = SOURCE_KEY
        .len()
        .checked_add(1)
        .and_then(|length| length.checked_add(digest.len()))
        .ok_or(Error::Overflow)?;
    key.try_reserve_exact(capacity)
        .map_err(|_| Error::Allocation)?;
    key.push_str(SOURCE_KEY);
    key.push(':');
    key.push_str(&digest);
    Ok(key)
}

pub(super) fn document_evidence(
    document: &CheckedRetainedResultDocumentV1<'_>,
) -> Result<Evidence> {
    evidence(
        document,
        &(
            INTERPRETATION,
            "unadmitted_document_projection",
            document.context_reference(),
            document.capture().body_sha256(),
        ),
    )
}

pub(super) fn row_evidence(
    document: &CheckedRetainedResultDocumentV1<'_>,
    row: &CheckedRetainedResultRowV1<'_>,
    target: &ProposedHtmlResultTargetV1<'_>,
    key: &str,
) -> Result<Evidence> {
    evidence(
        document,
        &RowNote {
            interpretation: INTERPRETATION,
            admission: "unadmitted_projection",
            content_key: key,
            context_reference: document.context_reference(),
            original_row_reference: row.reference(),
            original_row_digest: row.original_row_digest(),
            proposed_target: target,
        },
    )
}

#[derive(Serialize)]
struct RowNote<'a, 'target> {
    interpretation: &'a str,
    admission: &'a str,
    content_key: &'a str,
    context_reference: &'a str,
    original_row_reference: &'a str,
    original_row_digest: &'a str,
    proposed_target: &'a ProposedHtmlResultTargetV1<'target>,
}

fn evidence(
    document: &CheckedRetainedResultDocumentV1<'_>,
    note: &impl Serialize,
) -> Result<Evidence> {
    let bytes = codec::encode(note, 8192)?;
    let note = String::from_utf8(bytes).map_err(|_| Error::Invalid("encoded note is not UTF-8"))?;
    let source = SourceRef::new(
        INTERPRETATION,
        Some(document.capture().requested_url().into()),
    );
    let mut evidence = Evidence::parsed(source, document.capture().acquired_at().to_rfc3339());
    evidence.note = Some(note);
    Ok(evidence)
}
