use super::super::{
    codec,
    result_ownership::{CheckedRetainedResultDocumentV1, CheckedRetainedResultRowV1},
    MAX_PACKET,
};
use super::{
    HtmlResultProjectionError as Error, ProposedHtmlResultTargetV1, Result, MAX_PARTICIPATION_FACTS,
};
use census_domain::model::{EventKind, Mark, ProfileDate, Sport};

const ROW_INPUT_BYTES: usize = 4096;
const SHARED_OUTPUT_BYTES: usize = 16 * 1024;
const ROW_OUTPUT_OVERHEAD: usize = 8192;
const OUTPUT_COPY_FACTOR: usize = 16;

pub(super) fn all(
    document: &CheckedRetainedResultDocumentV1<'_>,
    targets: &[ProposedHtmlResultTargetV1<'_>],
) -> Result<()> {
    count(targets.len())?;
    if targets.len() != document.selected_count() {
        return Err(Error::Invalid(
            "targets do not cover every selected original row",
        ));
    }
    context(document)?;
    let mut encoded_ceiling = SHARED_OUTPUT_BYTES;
    for (index, target) in targets.iter().enumerate().take(MAX_PARTICIPATION_FACTS) {
        let row = document
            .row(index)
            .ok_or(Error::Invalid("selected original row missing"))?;
        let input_bytes = selected(&row, target)?;
        let needed = input_bytes
            .checked_mul(OUTPUT_COPY_FACTOR)
            .and_then(|bytes| bytes.checked_add(ROW_OUTPUT_OVERHEAD))
            .ok_or(Error::Overflow)?;
        encoded_ceiling = encoded_ceiling.checked_add(needed).ok_or(Error::Overflow)?;
        bound(
            encoded_ceiling,
            MAX_PACKET,
            "estimated encoded output bytes",
        )?;
        if let Some(previous) = index.checked_sub(1).and_then(|prior| targets.get(prior)) {
            let end = previous
                .locator
                .byte_offset
                .checked_add(previous.locator.byte_length)
                .ok_or(Error::Overflow)?;
            if target.locator.ordinal <= previous.locator.ordinal
                || target.locator.byte_offset < end
            {
                return Err(Error::Invalid(
                    "duplicate, overlapping or reordered targets",
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn count(needed: usize) -> Result<()> {
    if needed == 0 {
        return Err(Error::Invalid("no selected result rows"));
    }
    bound(needed, MAX_PARTICIPATION_FACTS, "selected rows")
}
pub(super) fn single(
    document: &CheckedRetainedResultDocumentV1<'_>,
    row_index: usize,
    target: &ProposedHtmlResultTargetV1<'_>,
) -> Result<()> {
    count(document.selected_count())?;
    context(document)?;
    let row = document
        .row(row_index)
        .ok_or(Error::Invalid("selected original row missing"))?;
    let input_bytes = selected(&row, target)?;
    let needed = input_bytes
        .checked_mul(OUTPUT_COPY_FACTOR)
        .and_then(|bytes| bytes.checked_add(ROW_OUTPUT_OVERHEAD))
        .and_then(|bytes| bytes.checked_add(SHARED_OUTPUT_BYTES))
        .ok_or(Error::Overflow)?;
    bound(needed, MAX_PACKET, "estimated encoded output bytes")
}

fn context(document: &CheckedRetainedResultDocumentV1<'_>) -> Result<()> {
    let page = document.parsed_page();
    text(&page.meet.name, 512)?;
    if page
        .region
        .as_deref()
        .and_then(census_domain::UsJurisdiction::parse)
        .is_none()
    {
        return Err(Error::Invalid("result jurisdiction unresolved"));
    }
    text(document.capture().requested_url(), 2048)?;
    text(document.meet_id(), 32)?;
    text(document.result_document_id(), 32)?;
    ProfileDate::parse(document.published_start())
        .map_err(|_| Error::Invalid("result requires an exact source day"))?;
    if page.meet.date != document.published_start()
        || document
            .published_end()
            .is_some_and(|end| end != document.published_start())
        || page
            .meet
            .end_date
            .as_deref()
            .is_some_and(|end| end != document.published_start())
        || !matches!(
            page.sport,
            Some(Sport::OutdoorTrack | Sport::IndoorTrack | Sport::CrossCountry)
        )
    {
        return Err(Error::Invalid("unsupported result date or sport scope"));
    }
    digest(document.context_reference())?;
    digest(document.capture().body_sha256())?;
    Ok(())
}

fn selected(
    row: &CheckedRetainedResultRowV1<'_>,
    target: &ProposedHtmlResultTargetV1<'_>,
) -> Result<usize> {
    source(row)?;
    let parsed = row.parsed_row();
    let event = row.event();
    for value in [
        target.subject_reference,
        target.proposed_attachment_reference,
        target.proposed_school_binding_reference,
    ] {
        digest(value)?;
    }
    canonical_id(target.proposed_athlete.as_str(), "ath_subject_")?;
    canonical_id(target.proposed_school.as_str(), "sch_")?;
    if target.locator != row.locator() || target.subject_reference != row.subject()?.reference() {
        return Err(Error::Invalid(
            "target differs from exact checked row subject",
        ));
    }
    Ok(codec::encode(
        &(
            target,
            parsed,
            &event.label,
            &event.kind,
            event.gender,
            &event.division,
            &event.round,
        ),
        ROW_INPUT_BYTES,
    )?
    .len())
}

fn source(row: &CheckedRetainedResultRowV1<'_>) -> Result<()> {
    let parsed = row.parsed_row();
    let event = row.event();
    for value in [
        Some(parsed.name.as_str()),
        Some(parsed.school.as_str()),
        Some(event.label.as_str()),
        parsed.heat.as_deref(),
        event.division.as_deref(),
        event.round.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        text(value, 512)?;
    }
    match &parsed.mark {
        Mark::Raw(value)
        | Mark::FieldImperial {
            feet_mark: value, ..
        } => text(value, 512)?,
        _ => {}
    }
    if event.kind.is_relay()
        || !parsed.legs.is_empty()
        || matches!(
            event.kind,
            EventKind::Pentathlon | EventKind::Heptathlon | EventKind::Decathlon
        )
    {
        return Err(Error::Invalid(
            "unsupported relay or combined-result projection",
        ));
    }
    if parsed.wind_mps.is_some_and(|wind| !wind.is_finite()) {
        return Err(Error::Invalid("nonfinite source wind"));
    }
    Ok(())
}

pub(super) fn identity(
    document: &CheckedRetainedResultDocumentV1<'_>,
    row: &CheckedRetainedResultRowV1<'_>,
    athlete: &census_domain::model::AthleteId,
) -> Result<()> {
    count(document.selected_count())?;
    context(document)?;
    source(row)?;
    canonical_id(athlete.as_str(), "ath_subject_")?;
    let event = row.event();
    let input = codec::encode(
        &(
            athlete,
            row.parsed_row(),
            &event.label,
            &event.kind,
            event.gender,
            &event.division,
            &event.round,
        ),
        ROW_INPUT_BYTES,
    )?
    .len();
    let ceiling = input
        .checked_mul(OUTPUT_COPY_FACTOR)
        .and_then(|bytes| bytes.checked_add(ROW_OUTPUT_OVERHEAD))
        .and_then(|bytes| bytes.checked_add(SHARED_OUTPUT_BYTES))
        .ok_or(Error::Overflow)?;
    bound(ceiling, MAX_PACKET, "estimated encoded output bytes")
}

pub(super) fn canonical_id(value: &str, prefix: &str) -> Result<()> {
    let suffix = value
        .strip_prefix(prefix)
        .ok_or(Error::Invalid("proposed canonical ID kind"))?;
    if suffix.len() != 16 || !suffix.bytes().all(hex) {
        return Err(Error::Invalid("proposed canonical ID shape"));
    }
    Ok(())
}

pub(super) fn digest(value: &str) -> Result<()> {
    if value.len() != 64 || !value.bytes().all(hex) {
        return Err(Error::Invalid("proposed reference is not a SHA256"));
    }
    Ok(())
}

fn hex(byte: u8) -> bool {
    byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
}

pub(super) fn text(value: &str, limit: usize) -> Result<()> {
    bound(value.len(), limit, "source text bytes")?;
    if value.is_empty() || value.chars().any(char::is_control) {
        return Err(Error::Invalid("source text empty or contains controls"));
    }
    Ok(())
}

pub(super) fn bound(needed: usize, limit: usize, resource: &'static str) -> Result<()> {
    if needed > limit {
        return Err(Error::Budget {
            resource,
            needed,
            limit,
        });
    }
    Ok(())
}
