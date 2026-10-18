use crate::replay::{ensure_rows, unmapped, Capture};
use anyhow::Result;

pub(super) fn replay(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    if file.starts_with("gomats.org__staff-directory") {
        let directory = census_crawl::sidearm_staff::parse_staff_directory(body)?;
        ensure_rows(file, directory.members.len(), "staff rows")?;
        let published_emails = directory
            .members
            .iter()
            .filter(|member| !member.email.is_empty())
            .count();
        return Ok(format!(
            "staff_directory name={:?} members={} published_emails={published_emails}",
            directory.name,
            directory.members.len()
        ));
    }
    unmapped("sidearm_staff", file)
}
