use super::super::{bound, invalid, markup, RetainedHtmlCaptureV1};
use super::{
    urls, CheckedLinkedCompetitionCategoryV1, CheckedMileSplitProfileSubjectV1,
    CompetitionCategoryEvidenceV1, ProviderTargetProofV1,
};
use census_domain::model::Gender;
use census_store::{StoreError, StoreResult};

mod context;

pub(super) fn read(
    capture: &RetainedHtmlCaptureV1,
    profile: &CheckedMileSplitProfileSubjectV1,
) -> StoreResult<CompetitionCategoryEvidenceV1> {
    let (meet, team) = urls::team_result(capture.requested_url())?;
    if team != profile.school.id {
        return Err(invalid(
            "linked result requested team differs from profile owner",
        ));
    }
    let html = capture.body()?;
    markup::canonical(&html, capture.requested_url())?;
    let context = context::read(capture, &html, profile)?;
    let selected = markup::elements(&html, b"tr", None)?.into_iter().try_fold(
        Vec::new(),
        |mut selected, row| -> StoreResult<Vec<_>> {
            if row.attribute("data-team-row").is_none() {
                return Ok(selected);
            }
            let cells = markup::elements(row.html, b"td", Some("meetTeams__participant"))?;
            let cell = markup::single(cells)?;
            let links = markup::elements(cell.html, b"a", None)?;
            let target_text = row.attribute("data-search") == Some(profile.name.as_str())
                || markup::text(cell.html)? == profile.name;
            if links.is_empty() {
                if target_text {
                    return Err(invalid(
                        "target-looking linked row has no participant ownership",
                    ));
                }
                return Ok(selected);
            }
            let link = markup::single(links)?;
            let person = urls::person(
                link.attribute("href")
                    .ok_or_else(|| invalid("linked result subject href absent"))?,
            )?;
            if person.id != profile.person.id {
                if target_text {
                    return Err(invalid(
                        "target-looking linked row has changed participant ownership",
                    ));
                }
                return Ok(selected);
            }
            if markup::text(link.html)? != profile.name
                || row.attribute("data-search") != Some(profile.name.as_str())
            {
                return Err(invalid("same-ID linked row subject name differs"));
            }
            bound(
                selected
                    .len()
                    .checked_add(1)
                    .ok_or(StoreError::CounterOverflow)?,
                64,
                "provider target same-ID rows",
            )?;
            let checked = checked_row(capture, &row, &meet, &context)?;
            selected
                .try_reserve(1)
                .map_err(|_| invalid("linked category row allocation refused"))?;
            selected.push(checked);
            Ok(selected)
        },
    )?;
    Ok(classify(selected))
}

fn checked_row(
    capture: &RetainedHtmlCaptureV1,
    row: &markup::Element<'_>,
    meet: &str,
    context: &context::Context,
) -> StoreResult<CheckedLinkedCompetitionCategoryV1> {
    let raw = row.attribute("data-gender");
    if let Some(value) = raw {
        bound(value.len(), 64, "linked competition category bytes")?;
    }
    let raw = raw.map(str::to_owned);
    let category = match raw.as_deref() {
        Some("m") => Some(Gender::Boys),
        Some("f") => Some(Gender::Girls),
        _ => None,
    };
    let event = urls::numeric(
        row.attribute("data-event")
            .ok_or_else(|| invalid("linked event code absent"))?,
    )?;
    let source = markup::single(markup::elements(
        row.html,
        b"td",
        Some("meetTeams__source"),
    )?)?;
    let source = markup::single(markup::elements(source.html, b"a", None)?)?;
    let result_document_url = source
        .attribute("href")
        .ok_or_else(|| invalid("linked result document absent"))?;
    let result_document_id = urls::result_document(result_document_url, meet)?;
    let mark = markup::single(markup::elements(
        row.html,
        b"span",
        Some("meetTeams__markValue"),
    )?)?;
    let mark = markup::text(mark.html)?;
    bound(mark.len(), 128, "linked result raw mark bytes")?;
    if mark.is_empty() {
        return Err(invalid("linked result raw mark empty"));
    }
    Ok(CheckedLinkedCompetitionCategoryV1 {
        raw,
        category,
        meet_id: meet.into(),
        result_document_id,
        result_document_url: result_document_url.into(),
        event_code: event.into(),
        mark,
        meet_name: context.name.clone(),
        meet_day: context.day.clone(),
        context: context.proof.clone(),
        school_context: context.school_proof.clone(),
        proof: ProviderTargetProofV1::new(capture, row.locator()),
    })
}

fn classify(rows: Vec<CheckedLinkedCompetitionCategoryV1>) -> CompetitionCategoryEvidenceV1 {
    let (category, missing, conflict) = rows.iter().fold(
        (None, rows.is_empty(), false),
        |(category, missing, conflict), row| match row.category {
            Some(value) => (
                Some(value),
                missing,
                conflict || category.is_some_and(|prior| prior != value),
            ),
            None => (category, true, conflict),
        },
    );
    if conflict {
        CompetitionCategoryEvidenceV1::Conflicted { rows }
    } else if missing {
        CompetitionCategoryEvidenceV1::Unavailable { rows }
    } else if let Some(category) = category {
        CompetitionCategoryEvidenceV1::Agreed { category, rows }
    } else {
        CompetitionCategoryEvidenceV1::Unavailable { rows }
    }
}
