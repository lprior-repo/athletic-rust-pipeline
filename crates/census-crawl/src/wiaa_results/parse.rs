use super::classify::ArtifactFormat;
use crate::result_file::ParsedMeet;

pub fn parse_result_body(
    body: &[u8],
    format: ArtifactFormat,
    source: SourceRef,
    year: i16,
) -> Option<ParsedMeet> {
    use ArtifactFormat::{HytekHtml, HytekText, Pdf, RaceDay, Unparsed};

    match format {
        HytekHtml => {
            let text = String::from_utf8_lossy(body);
            let lines = crate::hytek::lines_from_html(&text);
            crate::hytek::parse(&lines, source)
        }
        HytekText => {
            let text = String::from_utf8_lossy(body);
            let lines = crate::hytek::lines_from_text(&text);
            crate::hytek::parse(&lines, source)
        }
        RaceDay => {
            let text = String::from_utf8_lossy(body);
            crate::raceday::parse(&text, source, year).ok()
        }
        Pdf => match pdftotext(body) {
            Ok(text) => parse_pdf(&text, source, year).0,
            Err(_) => None,
        },
        Unparsed => None,
    }
}

use crate::CrawlResult;
use census_domain::model::SourceRef;
use std::time::Duration;

pub(super) fn parse_pdf(
    text: &str,
    source: SourceRef,
    archive_year: i16,
) -> (Option<crate::result_file::ParsedMeet>, Option<&'static str>) {
    let lines = crate::hytek::lines_from_pdf_text(text);
    if let Some(parsed) = crate::hytek::parse(&lines, source.clone()) {
        return (Some(parsed), Some("hytek"));
    }
    if let Some(parsed) = crate::compiled::parse(&lines, source.clone(), archive_year) {
        return (Some(parsed), Some("compiled"));
    }
    if let Some(parsed) = crate::xc::parse(&lines, source, archive_year) {
        return (Some(parsed), Some("xc"));
    }
    (None, None)
}

pub(super) fn pdftotext(body: &[u8]) -> CrawlResult<String> {
    pdftotext_with(
        "pdftotext",
        &["-layout", "-", "-"],
        body,
        crate::convert::CONVERTER_DEADLINE,
        crate::convert::CONVERTER_MAX_OUTPUT_BYTES,
    )
}

pub(super) fn pdftotext_with(
    command: &str,
    args: &[&str],
    body: &[u8],
    deadline: Duration,
    max_output: usize,
) -> CrawlResult<String> {
    let bytes =
        crate::convert::converter_stdout_capped(command, args, body, deadline, max_output)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}
