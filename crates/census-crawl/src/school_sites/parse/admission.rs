use super::{Rules, Signals};
use crate::{CrawlError, CrawlResult};

pub(super) fn page(rules: &Rules, url: &str, html: &str, signals: &Signals) -> CrawlResult<()> {
    check("school site body bytes", html.len(), 1024 * 1024)?;
    check("school site URL bytes", url.len(), 4096)?;
    check(
        "school site retained email rows",
        signals.emails.len(),
        16_384,
    )?;
    check(
        "school site retained coach rows",
        signals.coach_hits.len(),
        16_384,
    )?;
    check(
        "school site retained director rows",
        signals.ad_hits.len(),
        16_384,
    )?;
    check("school site retained pages", signals.pages.len(), 13)?;
    check(
        "school site email rows",
        rules.mailto.find_iter(html).take(4097).count(),
        4096,
    )?;
    check(
        "school site bare email rows",
        rules.bare_mail.find_iter(html).take(4097).count(),
        4096,
    )?;
    for pattern in rules.coach.iter().chain(&rules.ad) {
        check(
            "school site appointment rows",
            pattern.find_iter(html).take(1025).count(),
            1024,
        )?;
    }
    check(
        "school site tables",
        rules.table.find_iter(html).take(9).count(),
        8,
    )?;
    check(
        "school site table rows",
        rules.row.find_iter(html).take(321).count(),
        320,
    )?;
    Ok(())
}

fn check(resource: &'static str, requested: usize, limit: usize) -> CrawlResult<()> {
    if requested > limit {
        return Err(CrawlError::Resource {
            resource,
            requested,
            limit,
        });
    }
    Ok(())
}
