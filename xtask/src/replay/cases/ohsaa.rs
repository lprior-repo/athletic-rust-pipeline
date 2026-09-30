use crate::replay::{ensure_rows, unmapped, Capture};
use anyhow::Result;
use census_crawl::ohsaa;

pub(super) fn ohsaa(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    let refusal = matches!(
        file,
        "search_no_results.html" | "sports_malformed.html" | "ad_malformed.html"
    );
    if file.starts_with("search_") {
        let rows = ohsaa::parse_search(body).len();
        if !refusal {
            ensure_rows(file, rows, "search rows")?;
        }
        return Ok(format!("search rows={rows}"));
    }
    if file.starts_with("sports_") {
        let rows = ohsaa::parse_sports_table(body).len();
        if !refusal {
            ensure_rows(file, rows, "sport rows")?;
        }
        return Ok(format!("sports rows={rows}"));
    }
    if file.starts_with("ad_") {
        let page = ohsaa::parse_ad_page(body);
        if !refusal {
            ensure_rows(file, page.office_roles.len(), "AD rows")?;
        }
        let director = page
            .director
            .as_ref()
            .map_or("none", |(name, _)| name.as_str());
        return Ok(format!(
            "ad_page director={director:?} office_roles={}",
            page.office_roles.len()
        ));
    }
    unmapped("ohsaa", file)
}
