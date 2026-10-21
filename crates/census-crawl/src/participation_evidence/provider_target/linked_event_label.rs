use super::super::{bound, hash, invalid, markup};
use super::{ProviderTargetProofV1, SourceTargetPacketV1};
use census_domain::evidence_subject_attachment::{AttachmentDigestV1, AttachmentObjectReferenceV1};
use census_domain::model::SourceIdentity;
use census_store::StoreResult;
use std::ops::Range;

#[derive(Debug, Clone)]
pub struct CheckedLinkedEventLabelV1 {
    label: String,
    source_url: String,
    heading_proof: ProviderTargetProofV1,
    heading_reference: AttachmentObjectReferenceV1,
    row_reference: AttachmentObjectReferenceV1,
    table_proof: ProviderTargetProofV1,
    row_proof: ProviderTargetProofV1,
    person: SourceIdentity,
}

impl CheckedLinkedEventLabelV1 {
    pub fn label(&self) -> &str {
        &self.label
    }
    pub fn source_url(&self) -> &str {
        &self.source_url
    }
    pub fn heading_proof(&self) -> &ProviderTargetProofV1 {
        &self.heading_proof
    }
    pub fn heading_reference(&self) -> &AttachmentObjectReferenceV1 {
        &self.heading_reference
    }
    pub fn row_reference(&self) -> &AttachmentObjectReferenceV1 {
        &self.row_reference
    }
    pub fn table_proof(&self) -> &ProviderTargetProofV1 {
        &self.table_proof
    }
    pub fn row_proof(&self) -> &ProviderTargetProofV1 {
        &self.row_proof
    }
    pub fn person(&self) -> &SourceIdentity {
        &self.person
    }
    pub fn into_source_label(self) -> census_domain::model::SourceEventLabel {
        census_domain::model::SourceEventLabel {
            source: census_domain::model::SourceRef::new("milesplit_mi", Some(self.source_url)),
            label: self.label,
        }
    }
}

pub(super) fn read(
    packet: &SourceTargetPacketV1,
    row_proof: &ProviderTargetProofV1,
) -> StoreResult<CheckedLinkedEventLabelV1> {
    let intake = packet.read()?;
    let profile = intake.profile();
    let person = profile.person().clone();
    let capture = &packet.linked_results;
    let html = capture.body()?;
    markup::canonical(&html, capture.requested_url())?;

    let matched = match_linked_row(intake.categories(), row_proof)?;
    let row_span = parse_locator(matched.proof().locator())?;

    let row_html = html
        .get(row_span.start..row_span.end)
        .ok_or_else(|| invalid("row span exceeds linked results body"))?;
    let row_reference = literal_reference(row_html)?;

    let (heading, table, label) = heading_for_row(&html, &row_span)?;
    let heading_proof = ProviderTargetProofV1::new(capture, heading.locator());
    let heading_reference = literal_reference(heading.html)?;
    let table_proof = ProviderTargetProofV1::new(capture, table.locator());

    let mut source_url = String::new();
    source_url
        .try_reserve(capture.requested_url().len())
        .map_err(|_| invalid("linked event source URL allocation refused"))?;
    source_url.push_str(capture.requested_url());

    Ok(CheckedLinkedEventLabelV1 {
        label,
        source_url,
        heading_proof,
        heading_reference,
        row_reference,
        table_proof,
        row_proof: matched.proof().clone(),
        person,
    })
}

fn match_linked_row<'a>(
    categories: &'a super::CompetitionCategoryEvidenceV1,
    proof: &ProviderTargetProofV1,
) -> StoreResult<&'a super::CheckedLinkedCompetitionCategoryV1> {
    categories
        .rows()
        .iter()
        .try_fold(
            None,
            |matched, row| -> StoreResult<Option<&'a super::CheckedLinkedCompetitionCategoryV1>> {
                if row.proof().capture() == proof.capture()
                    && row.proof().interpretation() == proof.interpretation()
                    && row.proof().locator() == proof.locator()
                {
                    if matched.is_some() {
                        return Err(invalid(
                            "supplied event proof matches duplicate linked rows",
                        ));
                    }
                    return Ok(Some(row));
                }
                Ok(matched)
            },
        )?
        .ok_or_else(|| invalid("supplied event proof matches no linked row"))
}

fn parse_locator(locator: &str) -> StoreResult<Range<usize>> {
    let Some(tail) = locator.strip_prefix("html_bytes:") else {
        return Err(invalid("event row proof locator prefix missing"));
    };
    let (start_part, rest) = tail
        .split_once(':')
        .ok_or_else(|| invalid("event row proof locator shape invalid"))?;
    if rest.contains(':') {
        return Err(invalid("event row proof locator shape invalid"));
    }
    let start: usize = start_part
        .parse()
        .map_err(|_| invalid("event row proof locator start not numeric"))?;
    let end: usize = rest
        .parse()
        .map_err(|_| invalid("event row proof locator end not numeric"))?;
    if start >= end {
        return Err(invalid("event row proof locator start exceeds end"));
    }
    Ok(start..end)
}

fn heading_for_row<'a>(
    html: &'a str,
    row_span: &Range<usize>,
) -> StoreResult<(markup::Element<'a>, markup::Element<'a>, String)> {
    let table = table_containing_row(html, row_span)?;
    let group_id = parse_group_id(table.attribute("aria-labelledby"))?;
    let heading = heading_for_group(html, group_id)?;
    let label = markup::text(heading.html)?;
    bound(label.len(), 512, "linked event label bytes")?;
    if label.is_empty() {
        return Err(invalid("linked event label text empty"));
    }
    Ok((heading, table, label))
}

fn table_containing_row<'a>(
    html: &'a str,
    row_span: &Range<usize>,
) -> StoreResult<markup::Element<'a>> {
    markup::elements(html, b"table", Some("meetTeams__table"))?
        .into_iter()
        .try_fold(
            None,
            |selected, table| -> StoreResult<Option<markup::Element<'a>>> {
                if table.span.start <= row_span.start && row_span.end <= table.span.end {
                    if selected.is_some() {
                        return Err(invalid("result row contained in duplicate tables"));
                    }
                    return Ok(Some(table));
                }
                Ok(selected)
            },
        )?
        .ok_or_else(|| invalid("no result table contains the row"))
}

fn parse_group_id(raw: Option<&str>) -> StoreResult<&str> {
    let value = raw.ok_or_else(|| invalid("result table lacks event group label binding"))?;
    let mut tokens = value.split_ascii_whitespace();
    let first = tokens
        .next()
        .ok_or_else(|| invalid("result table group label binding empty"))?;
    if first.is_empty() {
        return Err(invalid("result table group label binding is ambiguous"));
    }
    if tokens.next().is_some() {
        return Err(invalid("result table group label binding is ambiguous"));
    }
    bound(first.len(), 128, "event group label id bytes")?;
    Ok(first)
}

fn heading_for_group<'a>(html: &'a str, id: &str) -> StoreResult<markup::Element<'a>> {
    let result = markup::elements(html, b"h3", Some("meetTeams__groupTitle"))?
        .into_iter()
        .try_fold(
            None,
            |selected, heading| -> StoreResult<Option<markup::Element<'a>>> {
                if heading.attribute("id") == Some(id) {
                    if selected.is_some() {
                        return Err(invalid("duplicate event group title id"));
                    }
                    return Ok(Some(heading));
                }
                Ok(selected)
            },
        )?;
    result.ok_or_else(|| invalid("no event group title matches table label"))
}

fn literal_reference(html: &str) -> StoreResult<AttachmentObjectReferenceV1> {
    let digest = hash(html.as_bytes());
    let bytes: u64 = html
        .len()
        .try_into()
        .map_err(|_| invalid("event reference bytes overflow"))?;
    Ok(AttachmentObjectReferenceV1 {
        digest: AttachmentDigestV1::parse(&digest)
            .map_err(census_store::EvidenceSubjectAttachmentStoreError::Domain)?,
        bytes,
    })
}

#[cfg(test)]
mod tests;
