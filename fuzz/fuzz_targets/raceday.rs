//! Fuzz target for the RaceDay Scoring parser.
//!
//! Exercises `raceday::parse`.
//! Input: raw HTML body of a RaceDay result page.

#![no_main]

use libfuzzer_sys::fuzz_target;
use census_crawl::raceday;
use census_domain::model::SourceRef;

fuzz_target!(|data: &[u8]| {
    let body = String::from_utf8_lossy(data);
    let source = SourceRef::new("fuzz", None);
    let year: i16 = (i16::from(data.first().copied().unwrap_or(0)) % 41) + 2020;
    let _ = raceday::parse(&body, source, year);
});
