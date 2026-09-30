use crate::replay::{ensure_rows, unmapped, Capture};
use anyhow::Result;
use census_crawl::plain_names;

pub(super) fn plain_names(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    if file == "nsaa_directory_form.html" {
        let names = plain_names::parse_nsaa_school_names(body)?;
        ensure_rows(file, names.len(), "school names")?;
        return Ok(format!("nsaa_directory_form names={}", names.len()));
    }
    if file.starts_with("nsaa_directory_") || file.starts_with("nsaa_school_get_") {
        let schools = plain_names::parse_nsaa_directory(body)?;
        ensure_rows(file, schools.len(), "directory rows")?;
        return Ok(format!("nsaa_directory schools={}", schools.len()));
    }
    if file == "nd_schools_index.html" {
        let refs = plain_names::parse_nd_school_refs(body)?;
        ensure_rows(file, refs.len(), "school refs")?;
        return Ok(format!("nd_schools_index schools={}", refs.len()));
    }
    if file.starts_with("nd_school_page") {
        let staff = plain_names::parse_nd_staff(body)?;
        let offerings = plain_names::parse_nd_offerings(body)?;
        ensure_rows(file, staff.len(), "staff rows")?;
        return Ok(format!(
            "nd_school_page staff={} offerings={}",
            staff.len(),
            offerings.len()
        ));
    }
    unmapped("plain_names", file)
}
