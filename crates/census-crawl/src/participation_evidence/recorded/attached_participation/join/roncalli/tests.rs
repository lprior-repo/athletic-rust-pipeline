use super::*;
use crate::milesplit::SourceRowLocator;
use crate::participation_evidence::packet::roncalli::tests as fixture;
use crate::participation_evidence::result_ownership::{
    read_retained_source_result_rows, CheckedHtmlResultSourceV1,
};
use fixture::TestResult;

const BODY: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/sources/national-aggregators/samples/directathletics-tfrrs/indiana-tfrrs-list-5489.html"));
const URL: &str = "https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List/2026/i";

fn locator() -> SourceRowLocator {
    SourceRowLocator {
        ordinal: 200,
        byte_offset: 313596,
        byte_length: 1349,
    }
}

fn held() -> TestResult<VerifiedIndependentParticipationV1> {
    fixture::read(&fixture::packet(fixture::REVIEW, fixture::SCHEDULE)?)
}

fn pair(
    original: &str,
    replace: &str,
    consume: impl FnOnce(&CheckedHtmlResultDocumentV1<'_>, &CheckedHtmlResultRowV1<'_>) -> TestResult,
) -> TestResult {
    let mut body = BODY.to_owned();
    let mut locator = locator();
    let end = locator
        .byte_offset
        .checked_add(locator.byte_length)
        .ok_or("row end overflow")?;
    let row = body
        .get(locator.byte_offset..end)
        .ok_or("physical Tessa row absent")?;
    let changed = row.replacen(original, replace, 1);
    if !row.contains(original) {
        return Err("controlled row mutation absent".into());
    }
    locator.byte_length = changed.len();
    body.replace_range(locator.byte_offset..end, &changed);
    let capture = fixture::capture(URL, body.as_bytes())?;
    let document =
        read_retained_source_result_rows(&capture, &[locator], CheckedHtmlResultSourceV1::Tfrrs)?;
    let checked = document.row(0).ok_or("checked source row absent")?;
    consume(&document, &checked)
}

#[test]
fn actual_converted_girls_row_matches_original_mark_and_requires_calendar_review() -> TestResult {
    let source = held()?;
    pair("Tessa Mize", "Tessa Mize", |document, checked| {
        check!(eq; applicability(document, checked, &source)?, Some(DocumentaryCohortAgreementV1::Agreed));
        let CheckedHtmlResultRowV1::Tfrrs(row) = checked else {
            return Err("TFRRS row absent".into());
        };
        check!(eq; row.original_row_digest(), "d33bf3fb6858f07f8e41576232afd0bf89c848e420fbe67bef779f0be6f1df8d");
        check!(eq; row.original_mark(), Some("11:29.42"));
        check!(eq; row.published_mark(), "11:23.56");
        check!(eq; row.event_label(), "3200 Meters");
        check!(eq; row.meet_name(), "Warren Central HSR Qualifier #2");
        check!(row.original_row()?.contains("/tie_break/5489/36792/44"));
        check!(eq; source.roncalli_event().ok_or("candidate absent")?.event().place, 2);
        check!(eq; source.venue(), "UIndy");
        check!(eq; calendar_requirement(), DocumentarySeasonApplicabilityV1::ReviewedCalendarRequired);
        check!(ne; calendar_requirement(), DocumentarySeasonApplicabilityV1::Exact);
        Ok(())
    })
}

#[test]
fn changed_physical_name_grade_school_or_original_mark_refuses() -> TestResult {
    for (from, to, expected) in [
        (
            ">Tessa Mize</a>",
            ">Other Person</a>",
            "Roncalli documentary TFRRS person day or grade differs",
        ),
        (
            ">JR</div>",
            ">SR</div>",
            "Roncalli documentary TFRRS person day or grade differs",
        ),
        (
            ">Roncalli</a>",
            ">Other School</a>",
            "Roncalli documentary TFRRS context differs",
        ),
        (
            "Converted from 11:29.42",
            "Converted from 11:29.43",
            "Roncalli documentary TFRRS original event or mark differs",
        ),
    ] {
        let source = held()?;
        pair(from, to, |document, checked| {
            match applicability(document, checked, &source) {
                Err(crate::CrawlError::Store(census_store::StoreError::Refused { detail })) => {
                    check!(eq; detail, expected);
                    Ok(())
                }
                other => {
                    Err(format!("expected exact row refusal {expected}, got {other:?}").into())
                }
            }
        })?;
    }
    Ok(())
}

#[test]
fn other_actual_day_is_inapplicable_instead_of_backdating_the_school_status() -> TestResult {
    let source = held()?;
    pair("Mar 11, 2026", "Mar 12, 2026", |document, checked| {
        check!(eq; applicability(document, checked, &source)?, None);
        Ok(())
    })
}

#[test]
fn source_year_and_original_event_disagreement_cannot_be_repaired_by_name_agreement() -> TestResult
{
    pair("Tessa Mize", "Tessa Mize", |document, checked| {
        let mut source = held()?;
        source.school_year = SchoolYear::new(2026).ok_or("controlled school year absent")?;
        match applicability(document, checked, &source) {
            Err(crate::CrawlError::Store(census_store::StoreError::Refused { detail })) => {
                check!(eq; detail, "Roncalli documentary TFRRS person day or grade differs");
            }
            other => return Err(format!("expected school year disagreement, got {other:?}").into()),
        }
        let changed = fixture::SCHEDULE.replace("in the 3200 of", "in the 1600 of");
        let source = fixture::read(&fixture::packet(fixture::REVIEW, &changed)?)?;
        match applicability(document, checked, &source) {
            Err(crate::CrawlError::Store(census_store::StoreError::Refused { detail })) => {
                check!(eq; detail, "Roncalli documentary TFRRS original event or mark differs");
            }
            other => return Err(format!("expected event disagreement, got {other:?}").into()),
        }
        Ok(())
    })
}
