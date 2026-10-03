#![no_main]

use census_crawl::raceday;
use census_domain::model::SourceRef;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let body = String::from_utf8_lossy(data);
    let source = SourceRef::new("fuzz", None);
    let year: i16 =
        (i16::from(data.first().copied().map_or(0, core::convert::identity)) % 41) + 2020;
    let _ = raceday::parse(&body, source, year);
});
