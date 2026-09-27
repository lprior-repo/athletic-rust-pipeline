
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
