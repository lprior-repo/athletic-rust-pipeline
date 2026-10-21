use super::*;
use crate::result_calendar_correspondence::source::test_fixture::*;
use census_domain::model::ProfileDate;

const BODY: &[u8] = b"0123456789abcdefghijklmnop";
const URL: &str = "https://in.tfrrs.org/lists/5489/Indiana/2026/i";

fn locator() -> SourceRowLocator {
    SourceRowLocator {
        ordinal: 200,
        byte_offset: 10,
        byte_length: 10,
    }
}

fn context(source: &ResultCalendarSourceClaimV1) -> Context<'_> {
    (
        "retained-tfrrs-row-context/v1",
        URL,
        source.result.body.digest.as_str(),
        0..BODY.len(),
        0..10,
        "94245",
        "2026-03-11",
        "3200 Meters",
        Gender::Girls,
    )
}

#[test]
fn exact_row_context_has_canonical_cold_roundtrip() -> TestResult {
    let source = source(BODY)?;
    let original = context(&source);
    let bytes = codec::encode(&original, &mut AttachmentReplayGrantV1::new())?;
    let cold: Context<'_> = codec::decode_bundle(&bytes, &mut AttachmentReplayGrantV1::new())?;
    check!(eq; cold, original);
    validate(&source, &cold, URL, BODY, locator())?;
    let mut changed = bytes;
    changed.push(b' ');
    check!(
        codec::decode_bundle::<Context<'_>>(&changed, &mut AttachmentReplayGrantV1::new()).is_err()
    );
    Ok(())
}

#[test]
fn exact_day_year_category_and_context_identity_cannot_be_rehashed() -> TestResult {
    let source = source(BODY)?;
    let valid = context(&source);
    for field in [
        "contract",
        "url",
        "digest",
        "day",
        "meet-leading-zero",
        "meet-zero",
        "category",
    ] {
        let mut wrong = context(&source);
        match field {
            "contract" => wrong.0 = "foreign/v1",
            "url" => wrong.1 = "https://other.test/list",
            "digest" => wrong.2 = "foreign-body",
            "day" => wrong.6 = "2026-03-21",
            "meet-leading-zero" => wrong.5 = "094245",
            "meet-zero" => wrong.5 = "0",
            "category" => wrong.8 = Gender::Boys,
            _ => return Err("unknown controlled tamper".into()),
        }
        let bytes = codec::encode(&wrong, &mut AttachmentReplayGrantV1::new())?;
        let decoded = codec::decode_bundle(&bytes, &mut AttachmentReplayGrantV1::new())?;
        check!(validate(&source, &decoded, URL, BODY, locator()).is_err());
    }
    let mut changed = source.clone();
    changed.day = ProfileDate::parse("2026-03-21")?;
    check!(validate(&changed, &valid, URL, BODY, locator()).is_err());
    changed = source.clone();
    changed.year = SchoolYear::new(2024).ok_or("school year")?;
    check!(validate(&changed, &valid, URL, BODY, locator()).is_err());
    changed = source.clone();
    changed.sport = Sport::OutdoorTrack;
    check!(validate(&changed, &valid, URL, BODY, locator()).is_err());
    Ok(())
}

#[test]
fn section_header_and_selected_global_row_must_be_bounded() -> TestResult {
    let source = source(BODY)?;
    let valid = context(&source);
    for row in [
        SourceRowLocator {
            ordinal: 0,
            ..locator()
        },
        SourceRowLocator {
            ordinal: 4097,
            ..locator()
        },
        SourceRowLocator {
            byte_length: 0,
            ..locator()
        },
        SourceRowLocator {
            byte_offset: usize::MAX,
            byte_length: 2,
            ..locator()
        },
        SourceRowLocator {
            byte_offset: 25,
            byte_length: 2,
            ..locator()
        },
        SourceRowLocator {
            byte_offset: 9,
            ..locator()
        },
    ] {
        check!(validate(&source, &valid, URL, BODY, row).is_err());
    }
    for (section, header) in [
        (0..27, 0..10),
        (10..10, 0..10),
        (1..26, 0..10),
        (0..26, 0..11),
    ] {
        let wrong = (
            valid.0, valid.1, valid.2, section, header, valid.5, valid.6, valid.7, valid.8,
        );
        check!(validate(&source, &wrong, URL, BODY, locator()).is_err());
    }
    let utf8 = "012345678éabcdefghijklmnop".as_bytes();
    let wrong = (
        valid.0,
        valid.1,
        valid.2,
        0..utf8.len(),
        0..9,
        valid.5,
        valid.6,
        valid.7,
        valid.8,
    );
    check!(validate(&source, &wrong, URL, utf8, locator()).is_err());
    Ok(())
}
