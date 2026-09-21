use crate::model::SourceRef;

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
pub(super) fn pdftotext(body: &[u8]) -> std::io::Result<String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new("pdftotext")
        .args(["-layout", "-", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let Some(mut stdin) = child.stdin.take() else {
        return Err(std::io::Error::other("stdin was piped"));
    };
    let payload = body.to_vec();
    // The child's own exit status below is the failure the caller reports; a failed write only
    // means `pdftotext` stopped reading, so the result is discarded rather than double-reported.
    // Dropping the closure also drops the pipe, which is what tells the child its input is complete.
    let writer = std::thread::spawn(move || drop(stdin.write_all(&payload)));
    let output = child.wait_with_output()?;
    writer.join().ok();
    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "pdftotext exited with {}",
            output.status
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
