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

use crate::{CrawlError, CrawlResult};
use census_domain::model::SourceRef;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

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
    let mut child = spawn_pdftotext()?;
    let Some(mut stdin) = child.stdin.take() else {
        return Err(CrawlError::Invariant {
            detail: "stdin was piped".to_string(),
        });
    };
    let payload = body.to_vec();
    let writer = std::thread::spawn(move || drop(stdin.write_all(&payload)));
    let output = child.wait_with_output().map_err(pdftotext_failed)?;
    writer.join().ok();
    if !output.status.success() {
        return Err(pdftotext_failed(std::io::Error::other(format!(
            "pdftotext exited with {}",
            output.status
        ))));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn spawn_pdftotext() -> CrawlResult<Child> {
    Command::new("pdftotext")
        .args(["-layout", "-", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(pdftotext_failed)
}

fn pdftotext_failed(source: std::io::Error) -> CrawlError {
    CrawlError::Io {
        path: PathBuf::from("pdftotext"),
        source,
    }
}
