//! Fuzz target for the Cross-Country parser.
//!
//! Exercises `xc::parse`.
//! Input: lines of an XC result file.

#![no_main]

use libfuzzer_sys::fuzz_target;
use census_crawl::xc;
use census_domain::model::SourceRef;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let lines: Vec<String> = text.lines().map(String::from).collect();
    let source = SourceRef::new("fuzz", None);
    // Archive year in 2020..=2060, derived from the first input byte.
    let year: i16 = (i16::from(data.first().copied().unwrap_or(0)) % 41) + 2020;
    let _ = xc::parse(&lines, source, year);
});
