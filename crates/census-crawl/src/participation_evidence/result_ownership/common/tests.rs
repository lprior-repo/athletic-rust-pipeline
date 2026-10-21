use super::*;
use census_domain::model::{ExactSeconds, Mark};

pub(in crate::participation_evidence) mod fixture;

type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const SOURCES: [CheckedHtmlResultSourceV1; 2] = [
    CheckedHtmlResultSourceV1::MileSplit,
    CheckedHtmlResultSourceV1::Tfrrs,
];

#[test]
fn milesplit_adapter_keeps_the_original_decoder_rows_and_proof_preimages() -> TestResult {
    let source = CheckedHtmlResultSourceV1::MileSplit;
    let capture = fixture::capture(source)?;
    let locators = fixture::locators(source);
    let direct = read_retained_html_result_rows(&capture, &locators)?;
    let common = read_retained_source_result_rows(&capture, &locators, source)?;
    let CheckedHtmlResultDocumentV1::MileSplit(checked) = &common else {
        return Err("MileSplit checked adapter branch".into());
    };
    check!(eq; common.source(), source);
    check!(std::ptr::eq(common.capture(), &capture));
    check!(eq; common.context_reference(), direct.context_reference());
    check!(eq; checked.parsed_page(), direct.parsed_page());
    check!(eq; checked.published_start(), "2026-05-15");
    check!(eq; common.selected_count(), 3);
    for (index, literal) in ["13.57", "14.10", "36.23"].into_iter().enumerate() {
        let row = common.row(index).ok_or("common MileSplit row")?;
        let original = direct.row(index).ok_or("original MileSplit row")?;
        check!(eq; row.source(), source);
        check!(eq; row.locator(), original.locator());
        check!(eq; row.reference(), original.reference());
        check!(eq; row.context_reference(), original.context_reference());
        check!(eq; row.original_row_digest(), original.original_row_digest());
        check!(eq; row.subject()?, original.subject()?);
        let CheckedHtmlResultRowV1::MileSplit(native) = row else {
            return Err("MileSplit checked row branch".into());
        };
        check!(eq; native.parsed_row().mark, Mark::TimeSeconds(ExactSeconds::parse(literal)?));
        check!(eq; native.parsed_row().name, "Zacchaeus Brocks");
    }
    check!(common.row(3).is_none());
    Ok(())
}

#[test]
fn genuine_tfrrs_body_controlled_transport_keeps_each_rows_native_meet_and_conversion() -> TestResult
{
    let source = CheckedHtmlResultSourceV1::Tfrrs;
    let capture = fixture::capture(source)?;
    let common = read_retained_source_result_rows(&capture, &fixture::locators(source), source)?;
    check!(eq; common.source(), source);
    check!(std::ptr::eq(common.capture(), &capture));
    check!(eq; capture.body_sha256(), "053ff7832326a6d27e2456155db7a9530682393fae94a9fcb9ccc9e88bfc8407");
    check!(eq; common.selected_count(), 2);
    let mut contexts = Vec::new();
    for (index, (meet, day, published, original)) in [
        ("95201", "2026-03-21", "9:15.75", "9:22.21"),
        ("93917", "2026-03-11", "9:16.30", "9:22.77"),
    ]
    .into_iter()
    .enumerate()
    {
        let row = common.row(index).ok_or("checked TFRRS row")?;
        check!(eq; row.source(), source);
        check!(eq; row.locator(), fixture::locators(source)[index]);
        check!(eq; row.subject()?.document_namespace(), "tfrrs_result_document_v1");
        check!(eq; row.subject()?.native_document_id(), "5489");
        check!(row.context_reference() != common.context_reference());
        contexts.push(row.context_reference().to_owned());
        let CheckedHtmlResultRowV1::Tfrrs(native) = row else {
            return Err("TFRRS checked row branch".into());
        };
        check!(eq; native.meet_id(), meet);
        check!(eq; native.published_date(), day);
        check!(eq; native.published_mark(), published);
        check!(eq; native.original_mark(), Some(original));
        check!(eq; native.conversion_note(), Some(format!("Converted from {original} for Track Size.").as_str()));
    }
    check!(contexts.first() != contexts.get(1));
    check!(common.row(2).is_none());
    Ok(())
}

#[test]
fn cold_capture_reopen_keeps_both_checked_source_proofs_exact() -> TestResult {
    for source in SOURCES {
        let capture = fixture::capture(source)?;
        let locators = fixture::locators(source);
        let common = read_retained_source_result_rows(&capture, &locators, source)?;
        let before = proof(&common)?;
        let encoded = serde_json::to_vec(&capture)?;
        let reopened: RetainedHtmlCaptureV1 = serde_json::from_slice(&encoded)?;
        let after = read_retained_source_result_rows(&reopened, &locators, source)?;
        check!(eq; proof(&after)?, before);
    }
    Ok(())
}

#[test]
fn an_explicit_wrong_source_never_falls_back_to_the_other_decoder() -> TestResult {
    for source in SOURCES {
        let capture = fixture::capture(source)?;
        let requested = fixture::locators(source);
        let other = match source {
            CheckedHtmlResultSourceV1::MileSplit => CheckedHtmlResultSourceV1::Tfrrs,
            CheckedHtmlResultSourceV1::Tfrrs => CheckedHtmlResultSourceV1::MileSplit,
        };
        check!(read_retained_source_result_rows(&capture, &requested, other).is_err());
        check!(eq; read_retained_source_result_rows(&capture, &requested, source)?.source(), source);
    }
    Ok(())
}

#[test]
fn corrupt_capture_and_altered_or_repeated_locators_are_refused_for_both_sources() -> TestResult {
    for source in SOURCES {
        let capture = fixture::capture(source)?;
        let requested = fixture::locators(source);
        let original = proof(&read_retained_source_result_rows(
            &capture, &requested, source,
        )?)?;
        let mut wire = serde_json::to_value(&capture)?;
        wire["body_sha256"] = serde_json::json!("0".repeat(64));
        let corrupt = serde_json::from_value(wire)?;
        check!(matches!(
            read_retained_source_result_rows(&corrupt, &requested, source),
            Err(super::super::RetainedResultOwnershipError::Capture(_))
        ));
        for field in 0..3 {
            let mut altered = requested.clone();
            let row = altered.first_mut().ok_or("first original locator")?;
            match field {
                0 => row.ordinal += 1,
                1 => row.byte_offset += 1,
                _ => row.byte_length -= 1,
            }
            check!(read_retained_source_result_rows(&capture, &altered, source).is_err());
        }
        let first = *requested.first().ok_or("original locator")?;
        check!(read_retained_source_result_rows(&capture, &[first, first], source).is_err());
        check!(eq; proof(&read_retained_source_result_rows(&capture, &requested, source)?)?, original);
    }
    Ok(())
}

fn proof(document: &CheckedHtmlResultDocumentV1<'_>) -> TestResult<Vec<u8>> {
    let mut rows = Vec::new();
    for index in 0..document.selected_count() {
        let row = document.row(index).ok_or("observed common row")?;
        let subject = row.subject()?;
        rows.push(serde_json::json!({
            "locator":row.locator(),"reference":row.reference(),
            "context":row.context_reference(),"row_digest":row.original_row_digest(),
            "namespace":subject.document_namespace(),"native_id":subject.native_document_id(),
            "url":subject.document_url(),"body":subject.body_sha256(),
            "span":subject.locator(),"name":subject.name(),"school":subject.school(),
            "subject":subject.reference()
        }));
    }
    Ok(serde_json::to_vec(&serde_json::json!({
        "source":format!("{:?}",document.source()),"capture":document.capture(),
        "context":document.context_reference(),"rows":rows
    }))?)
}
