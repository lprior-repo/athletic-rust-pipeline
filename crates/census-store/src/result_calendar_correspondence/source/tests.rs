use super::*;
use census_domain::model::{Gender, Sport};
use test_fixture::*;

#[test]
fn exact_closed_parser_and_source_namespace_must_agree() -> TestResult {
    for parser in [
        ResultCalendarSourceParserV1::MileSplitDcc,
        ResultCalendarSourceParserV1::TfrrsRoncalli,
    ] {
        let mut source = source(b"controlled original")?;
        source.parser_contract = reference(parser.contract())?;
        (source.sport, source.category) = match parser {
            ResultCalendarSourceParserV1::MileSplitDcc => (Sport::OutdoorTrack, Gender::Boys),
            ResultCalendarSourceParserV1::TfrrsRoncalli => (Sport::IndoorTrack, Gender::Girls),
        };
        let materials = [material(parser.contract())?];
        let mut subject = subject(&source)?;
        subject.namespace = census_domain::evidence_subject_attachment::AttachmentTextV1::parse(
            parser.result_namespace(),
        )?;
        check!(eq; require_result_parser(&source, &materials, &subject, &mut AttachmentReplayGrantV1::new())?, parser);
        let wrong_namespace = match parser {
            ResultCalendarSourceParserV1::MileSplitDcc => "tfrrs_result_document_v1",
            ResultCalendarSourceParserV1::TfrrsRoncalli => "milesplit_result_document_v1",
        };
        subject.namespace =
            census_domain::evidence_subject_attachment::AttachmentTextV1::parse(wrong_namespace)?;
        let mut grant = AttachmentReplayGrantV1::new();
        check!(guarded(&mut grant, |grant| require_result_parser(
            &source, &materials, &subject, grant
        ))
        .is_err());
        check!(grant.ensure_available().is_err());
    }
    Ok(())
}

#[test]
fn rehashed_contract_or_wrong_supported_source_scope_is_refused() -> TestResult {
    let mut source = source(b"controlled original")?;
    let subject = subject(&source)?;
    let mut changed = RESULT_CALENDAR_TFRRS_RONCALLI_PARSER_CONTRACT_V1.to_vec();
    changed.push(b' ');
    source.parser_contract = reference(&changed)?;
    check!(selected_parser(
        &source,
        &[material(&changed)?],
        &mut AttachmentReplayGrantV1::new()
    )
    .is_err());
    source.parser_contract = reference(RESULT_CALENDAR_TFRRS_RONCALLI_PARSER_CONTRACT_V1)?;
    source.category = Gender::Boys;
    check!(require_result_parser(
        &source,
        &[material(RESULT_CALENDAR_TFRRS_RONCALLI_PARSER_CONTRACT_V1)?],
        &subject,
        &mut AttachmentReplayGrantV1::new()
    )
    .is_err());
    Ok(())
}
