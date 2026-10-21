use super::{CheckedRetainedTfrrsListV1, CheckedRetainedTfrrsRowV1, Error, Result, INTERPRETATION, SOURCE_KEY};
use crate::participation_evidence::{codec, hash, physical_result::ProposedHtmlResultTargetV1};
use census_domain::model::{Evidence, SourceRef};
use serde::Serialize;

pub(super) fn source_key(list: &CheckedRetainedTfrrsListV1<'_>, row: &CheckedRetainedTfrrsRowV1<'_>) -> Result<String> {
    let bytes = codec::encode(&(
        SOURCE_KEY, list.list_id(), row.meet_id(), row.performance_id(),
        list.capture().requested_url(), list.capture().body_sha256(),
        row.locator(), row.context_reference(),
    ), 4096)?;
    let digest = hash(&bytes);
    let capacity = SOURCE_KEY.len().checked_add(1)
        .and_then(|length| length.checked_add(digest.len())).ok_or(Error::Overflow)?;
    let mut key = String::new();
    key.try_reserve_exact(capacity).map_err(|_| Error::Allocation)?;
    key.push_str(SOURCE_KEY);
    key.push(':');
    key.push_str(&digest);
    Ok(key)
}

pub(super) fn meet_evidence(list: &CheckedRetainedTfrrsListV1<'_>, row: &CheckedRetainedTfrrsRowV1<'_>) -> Result<Evidence> {
    evidence(list, &(
        INTERPRETATION, "unadmitted_document_projection", list.context_reference(),
        row.context_reference(), list.capture().body_sha256(), row.meet_id(),
        row.meet_path(), row.meet_name(), row.published_date(), row.meet_span(), row.date_span(),
    ))
}

pub(super) fn row_evidence(
    list: &CheckedRetainedTfrrsListV1<'_>,
    row: &CheckedRetainedTfrrsRowV1<'_>,
    target: &ProposedHtmlResultTargetV1<'_>,
    key: &str,
) -> Result<Evidence> {
    evidence(list, &RowNote {
        interpretation: INTERPRETATION,
        admission: "unadmitted_projection",
        content_key: key,
        list_context_reference: list.context_reference(),
        context_reference: row.context_reference(),
        original_row_reference: row.reference(),
        original_row_digest: row.original_row_digest(),
        published_mark: row.published_mark(),
        original_mark: row.original_mark(),
        conversion_note: row.conversion_note(),
        published_mark_span: row.published_mark_span(),
        conversion_note_span: row.conversion_note_span(),
        proposed_target: target,
    })
}

#[derive(Serialize)]
struct RowNote<'a, 'target> {
    interpretation: &'a str,
    admission: &'a str,
    content_key: &'a str,
    list_context_reference: &'a str,
    context_reference: &'a str,
    original_row_reference: &'a str,
    original_row_digest: &'a str,
    published_mark: &'a str,
    original_mark: Option<&'a str>,
    conversion_note: Option<&'a str>,
    published_mark_span: std::ops::Range<usize>,
    conversion_note_span: Option<std::ops::Range<usize>>,
    proposed_target: &'a ProposedHtmlResultTargetV1<'target>,
}

fn evidence(list: &CheckedRetainedTfrrsListV1<'_>, note: &impl Serialize) -> Result<Evidence> {
    let note = String::from_utf8(codec::encode(note, 8192)?)
        .map_err(|_| Error::Invalid("encoded TFRRS note is not UTF-8"))?;
    let source = SourceRef::new(INTERPRETATION, Some(list.capture().requested_url().into()));
    let mut evidence = Evidence::parsed(source, list.capture().acquired_at().to_rfc3339());
    evidence.note = Some(note);
    Ok(evidence)
}
