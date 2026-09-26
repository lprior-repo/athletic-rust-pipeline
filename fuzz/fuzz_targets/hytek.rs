//! Fuzz target for the Hy-Tek parser.
//!
//! Exercises `hytek::lines_from_html`, `hytek::lines_from_text`, and `hytek::parse`.
//! Input: raw bytes that are split into lines (text or HTML) and fed to the Hy-Tek parser.

#![no_main]

use libfuzzer_sys::fuzz_target;
use census_crawl::hytek;
use census_domain::model::SourceRef;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data).into_owned();

    let lines_text = hytek::lines_from_text(&text);
    let source = SourceRef::new("fuzz", None);
    let _ = hytek::parse(&lines_text, source.clone());

    let lines_html = hytek::lines_from_html(&text);
    let _ = hytek::parse(&lines_html, source);
});
