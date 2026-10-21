use super::*;
use crate::milesplit::SourceRowLocator;
use crate::participation_evidence::result_ownership::{
    common_fixture, tfrrs::read_retained_tfrrs_list_rows, CheckedHtmlResultSourceV1,
};
use crate::participation_evidence::RetainedHtmlCaptureV1;

type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn genuine_full_tfrrs_rows_expose_only_exact_original_team_text_inside_owned_anchor() -> TestResult
{
    let capture = common_fixture::capture(CheckedHtmlResultSourceV1::Tfrrs)?;
    let checked = read_retained_tfrrs_list_rows(&capture, &locators())?;
    for (index, expected) in ["Columbus North", "Hamilton Southeastern"]
        .into_iter()
        .enumerate()
    {
        let row = checked.row(index).ok_or("genuine source row")?;
        let quote = read_original_team_text(&row)?;
        check!(eq; quote.text(), expected);
        check!(eq; quote.text(), row.team_name());
        check!(eq; checked.original_document().get(quote.span()), Some(expected));
        check!(quote.span().start > row.team_span().start);
        check!(quote.span().end < row.team_span().end);
        check!(quote.span().start >= row.locator().byte_offset);
        check!(
            quote.span().end
                <= row
                    .locator()
                    .byte_offset
                    .checked_add(row.locator().byte_length)
                    .ok_or("bounded row end")?
        );
        check!(quote.text().len() <= MAX_QUOTE_BYTES);
    }
    Ok(())
}

#[test]
fn literal_team_quote_preserves_existing_ownership_preimages_and_cold_replay() -> TestResult {
    let capture = common_fixture::capture(CheckedHtmlResultSourceV1::Tfrrs)?;
    let cold: RetainedHtmlCaptureV1 = serde_json::from_slice(&serde_json::to_vec(&capture)?)?;
    let original = read_retained_tfrrs_list_rows(&capture, &locators())?;
    let reopened = read_retained_tfrrs_list_rows(&cold, &locators())?;
    for index in 0..2 {
        let row = original.row(index).ok_or("original row")?;
        let before = (
            row.reference(),
            row.context_reference(),
            row.original_row_digest(),
            row.locator(),
        );
        let quote = read_original_team_text(&row)?;
        let next = reopened.row(index).ok_or("cold row")?;
        check!(eq; quote, read_original_team_text(&next)?);
        check!(eq; before, (row.reference(), row.context_reference(), row.original_row_digest(), row.locator()));
        check!(eq; before, (next.reference(), next.context_reference(), next.original_row_digest(), next.locator()));
    }
    Ok(())
}

#[test]
fn checked_normalized_team_with_nested_markup_cannot_issue_original_text_quote() -> TestResult {
    for replacement in [
        "<b>Columbus North</b>",
        "Columbus <span>North</span>",
        "Columbus<!--controlled split--> North",
    ] {
        let (capture, locator) = controlled_team_text(replacement)?;
        let checked = read_retained_tfrrs_list_rows(&capture, &[locator])?;
        let row = checked.row(0).ok_or("controlled checked team")?;
        check!(eq; row.team_name(), "Columbus North");
        check!(matches!(
            read_original_team_text(&row),
            Err(TfrrsTeamTextError::AmbiguousMarkup)
        ));
    }
    Ok(())
}

#[test]
fn checked_normalized_team_with_entity_or_whitespace_cannot_issue_original_text_quote() -> TestResult
{
    for replacement in [
        "Columbus&#32;North",
        "Columbus&#x20;North",
        " Columbus North ",
        "Columbus  North",
        "Columbus\nNorth",
        "Columbus\tNorth",
    ] {
        let (capture, locator) = controlled_team_text(replacement)?;
        let checked = read_retained_tfrrs_list_rows(&capture, &[locator])?;
        let row = checked.row(0).ok_or("controlled checked team")?;
        check!(eq; row.team_name(), "Columbus North");
        check!(matches!(
            read_original_team_text(&row),
            Err(TfrrsTeamTextError::TransformedText) | Err(TfrrsTeamTextError::AmbiguousMarkup)
        ));
    }
    Ok(())
}

#[test]
fn literal_token_capacity_is_exact_and_ambiguous_elements_never_yield_a_quote() -> TestResult {
    let exact = "X".repeat(MAX_QUOTE_BYTES);
    let html = format!("<a href=\"/teams/tf/controlled_m.html\">{exact}</a>");
    check!(eq; html.get(literal_span(&html, &exact)?), Some(exact.as_str()));
    let over = "X".repeat(MAX_QUOTE_BYTES.checked_add(1).ok_or("quote boundary")?);
    check!(matches!(
        literal_span(&format!("<a>{over}</a>"), &over),
        Err(TfrrsTeamTextError::Capacity {
            needed: 513,
            available: 512
        })
    ));
    for html in [
        "<a></a>",
        "<a>X</a><a>X</a>",
        "<a><b>X</b></a>",
        "<a>X<!--split--></a>",
        "X",
        "<a/>X",
        "<a>X",
        "<span>X</span>",
    ] {
        check!(literal_span(html, "X").is_err());
    }
    check!(matches!(
        literal_span("<a>X</a>", "Y"),
        Err(TfrrsTeamTextError::TransformedText)
    ));
    Ok(())
}

fn locators() -> Vec<SourceRowLocator> {
    common_fixture::locators(CheckedHtmlResultSourceV1::Tfrrs)
}

fn controlled_team_text(
    replacement: &str,
) -> TestResult<(RetainedHtmlCaptureV1, SourceRowLocator)> {
    let capture = common_fixture::capture(CheckedHtmlResultSourceV1::Tfrrs)?;
    let checked = read_retained_tfrrs_list_rows(&capture, &locators())?;
    let row = checked.row(0).ok_or("controlled original row")?;
    let quote = read_original_team_text(&row)?;
    let mut body = checked.original_document().to_owned();
    body.replace_range(quote.span(), replacement);
    let mut locator = row.locator();
    locator.byte_length = locator
        .byte_length
        .checked_sub(quote.text().len())
        .and_then(|bytes| bytes.checked_add(replacement.len()))
        .ok_or("controlled row bytes")?;
    let changed = RetainedHtmlCaptureV1::new(
        capture.requested_url(),
        capture.requested_url(),
        200,
        capture.acquired_at(),
        body.as_bytes(),
        capture.provenance().clone(),
    )?;
    Ok((changed, locator))
}
