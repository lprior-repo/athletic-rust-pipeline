//! Pure layout dispatch for WIAA result artifacts.
//!
//! This module exposes a single public entry point — [`parse_result_body`] — that routes an
//! artifact body to the correct parser based on its declared format. It is the seam the fuzz
//! lane needs: no network, no file I/O, no clock, no randomness. The only side effect is the
//! external `pdftotext` call for `ArtifactFormat::Pdf`, which the caller can handle separately
//! if pure dispatch is required.

use super::classify::ArtifactFormat;
use crate::result_file::ParsedMeet;

/// Dispatch one artifact body to the parser its format selects.
///
/// This is the pure seam the fuzz lane needs: given raw bytes and a declared format, the function
/// routes to the correct parser without any network, file, or clock dependency. For
/// [`ArtifactFormat::Pdf`] the external `pdftotext` tool is invoked (returning `None` on failure);
/// all other arms are purely in-process.
///
/// The `body` parameter is raw bytes — for text formats the function decodes via
/// `from_utf8_lossy`, and for PDF it is passed directly to `pdftotext`.
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

/// Read a PDF release with the vendor parsers, most specific first.
///
/// The WIAA archive publishes PDFs from several timers: Hy-Tek's own reports, the "Compiled" export
/// the association posts for meets without a Hy-Tek file, and cross-country files from Hy-Tek block
/// and AccuRace layouts. Each parser states its own header requirement, so the first one that
/// returns events owns the file and the runner reports which layout won.
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

/// Read a PDF release through `pdftotext -layout`.
///
/// The PDF is streamed in and the text out, so no temporary file is written; the writer runs on its
/// own thread because a large PDF exceeds the pipe buffer while the parent is still reading.
///
/// Nothing here opens a file: the tool itself is what the [`CrawlError::Io`] diagnostics name, and
/// a tool that is missing or that exits non-zero is an error the caller reports rather than a meet.
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

/// Start `pdftotext -layout`, reading the PDF from stdin and writing the text to stdout.
fn spawn_pdftotext() -> CrawlResult<Child> {
    Command::new("pdftotext")
        .args(["-layout", "-", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(pdftotext_failed)
}

/// The failure one `pdftotext` step reports: the tool itself is the path the diagnostic names.
fn pdftotext_failed(source: std::io::Error) -> CrawlError {
    CrawlError::Io {
        path: PathBuf::from("pdftotext"),
        source,
    }
}
