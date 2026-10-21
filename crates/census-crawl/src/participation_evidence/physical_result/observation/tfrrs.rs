use super::{Error, Result};
use census_domain::model::{CanonicalPerformance, Evidence, EvidenceMethod};
use serde::{Deserialize, Deserializer};

pub(super) fn matches(
    actual: &CanonicalPerformance,
    projected: &CanonicalPerformance,
    baseline: &Evidence,
) -> Result<bool> {
    if actual.evidence.is_empty()
        || baseline.method != EvidenceMethod::Parsed
        || baseline.source.url.as_deref().is_none_or(str::is_empty)
    {
        return Ok(false);
    }
    let expected = parse(baseline, &projected.source_key)?;
    for evidence in actual
        .evidence
        .iter()
        .chain(projected.evidence.iter().skip(1))
    {
        if evidence.source != baseline.source
            || evidence.method != baseline.method
            || evidence.observed_on != baseline.observed_on
        {
            return Ok(false);
        }
        if !immutable_match(&parse(evidence, &projected.source_key)?, &expected) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn parse<'a>(evidence: &'a Evidence, source_key: &str) -> Result<RowNote<'a>> {
    let note = evidence
        .note
        .as_deref()
        .ok_or(Error::Invalid("TFRRS physical source evidence note absent"))?;
    if note.len() > 8192 {
        return Err(Error::Budget {
            resource: "TFRRS physical source evidence note",
            needed: note.len(),
            limit: 8192,
        });
    }
    let parsed: RowNote<'_> = serde_json::from_str(note)
        .map_err(|_| Error::Invalid("TFRRS physical source evidence note malformed"))?;
    if parsed.interpretation != super::super::tfrrs::INTERPRETATION
        || parsed.admission != "unadmitted_projection"
        || parsed.content_key != source_key
    {
        return Err(Error::Invalid(
            "TFRRS physical source evidence interpretation differs",
        ));
    }
    super::super::admission::digest(parsed.proposed_target.proposed_attachment_reference)?;
    super::super::admission::digest(parsed.proposed_target.proposed_school_binding_reference)?;
    Ok(parsed)
}

fn immutable_match(left: &RowNote<'_>, right: &RowNote<'_>) -> bool {
    left.list_context_reference == right.list_context_reference
        && left.context_reference == right.context_reference
        && left.original_row_reference == right.original_row_reference
        && left.original_row_digest == right.original_row_digest
        && left.published_mark == right.published_mark
        && left.original_mark == right.original_mark
        && left.conversion_note == right.conversion_note
        && left.published_mark_span == right.published_mark_span
        && left.conversion_note_span == right.conversion_note_span
        && left.proposed_target.locator == right.proposed_target.locator
        && left.proposed_target.subject_reference == right.proposed_target.subject_reference
        && left.proposed_target.proposed_athlete == right.proposed_target.proposed_athlete
        && left.proposed_target.proposed_school == right.proposed_target.proposed_school
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RowNote<'a> {
    interpretation: &'a str,
    admission: &'a str,
    content_key: &'a str,
    list_context_reference: &'a str,
    context_reference: &'a str,
    original_row_reference: &'a str,
    original_row_digest: &'a str,
    published_mark: &'a str,
    #[serde(borrow, deserialize_with = "required_option")]
    original_mark: Option<&'a str>,
    #[serde(borrow, deserialize_with = "required_option")]
    conversion_note: Option<&'a str>,
    published_mark_span: ExactSpan,
    #[serde(deserialize_with = "required_option")]
    conversion_note_span: Option<ExactSpan>,
    #[serde(borrow)]
    proposed_target: ProposedTarget<'a>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProposedTarget<'a> {
    locator: ExactLocator,
    subject_reference: &'a str,
    proposed_athlete: &'a str,
    proposed_school: &'a str,
    proposed_attachment_reference: &'a str,
    proposed_school_binding_reference: &'a str,
}

#[derive(Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct ExactSpan {
    start: usize,
    end: usize,
}

#[derive(Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct ExactLocator {
    ordinal: u32,
    byte_offset: usize,
    byte_length: usize,
}

fn required_option<'de, D, T>(deserializer: D) -> std::result::Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}
