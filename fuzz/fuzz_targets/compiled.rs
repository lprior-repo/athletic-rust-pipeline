//! Fuzz target for the Compiled parser (WIAA compiled results).
//!
//! Exercises `compiled::parse`.
//! Input: lines of a compiled result file.

#![no_main]

use libfuzzer_sys::fuzz_target;
use census_crawl::compiled;
use census_domain::model::SourceRef;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let lines: Vec<String> = text.lines().map(String::from).collect();
    let source = SourceRef::new("fuzz", None);
    let year: i16 = (i16::from(data.first().copied().unwrap_or(0)) % 41) + 2020;
    let _ = compiled::parse(&lines, source, year);
});
