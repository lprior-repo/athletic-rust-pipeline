use crate::sources::{CrawlError, CrawlResult};
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
) -> (
    Option<crate::sources::result_file::ParsedMeet>,
    Option<&'static str>,
) {
    let lines = crate::sources::hytek::lines_from_pdf_text(text);
    if let Some(parsed) = crate::sources::hytek::parse(&lines, source.clone()) {
        return (Some(parsed), Some("hytek"));
    }
    if let Some(parsed) = crate::sources::compiled::parse(&lines, source.clone(), archive_year) {
        return (Some(parsed), Some("compiled"));
    }
    if let Some(parsed) = crate::sources::xc::parse(&lines, source, archive_year) {
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
    // The child's own exit status below is the failure the caller reports; a failed write only
    // means `pdftotext` stopped reading, so the result is discarded rather than double-reported.
    // Dropping the closure also drops the pipe, which is what tells the child its input is complete.
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
