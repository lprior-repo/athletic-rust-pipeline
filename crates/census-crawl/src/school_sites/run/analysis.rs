use super::Run;
use crate::school_sites::{analyse, Signals};
use crate::{CrawlError, CrawlResult};

const MAX_DIAGNOSTIC_BYTES: usize = 1024 * 1024;
const EXHAUSTED: &str = "diagnostic_budget_exhausted: stopped this site's remaining research; prior diagnostics retained, additional failure detail omitted\n";

pub(super) enum DiagnosticState {
    Collecting,
    Exhausted,
}

impl DiagnosticState {
    pub(super) fn exhausted(&self) -> bool {
        matches!(self, Self::Exhausted)
    }
}

impl Run<'_> {
    pub(super) fn analyse_page(&mut self, url: &str, html: &str) -> CrawlResult<bool> {
        let mut page = Signals::default();
        let result = analyse(self.rules, url, html, &mut page)
            .and_then(|()| merge(&mut self.outcome.signals, page));
        match result {
            Ok(()) => Ok(true),
            Err(error @ CrawlError::Schema { .. }) => self.parse_failure(url, "schema", error),
            Err(error @ CrawlError::Decode { .. }) => self.parse_failure(url, "decode", error),
            Err(error @ CrawlError::ParserBudget { .. }) => {
                self.parse_failure(url, "resource", error)
            }
            Err(error @ CrawlError::Resource { .. }) => self.parse_failure(url, "resource", error),
            Err(error) => Err(error),
        }
    }

    fn parse_failure(&mut self, url: &str, kind: &str, error: CrawlError) -> CrawlResult<bool> {
        self.record_issue(url, &format!("{kind}: {error}"))?;
        Ok(false)
    }

    pub(super) fn record_issue(&mut self, url: &str, detail: &str) -> CrawlResult<()> {
        if self.diagnostics.exhausted() {
            return Ok(());
        }
        let message = format!("{url}: {detail}\n");
        let previous = self.outcome.note.as_ref().map_or(0, String::len);
        let requested =
            previous
                .checked_add(message.len())
                .ok_or_else(|| CrawlError::Arithmetic {
                    detail: "school site diagnostic byte counter overflow".to_owned(),
                })?;
        let detail_limit = MAX_DIAGNOSTIC_BYTES
            .checked_sub(EXHAUSTED.len())
            .ok_or_else(|| CrawlError::Invariant {
                detail: "school diagnostic exhaustion marker exceeds budget".to_owned(),
            })?;
        let message = if requested > detail_limit {
            self.diagnostics = DiagnosticState::Exhausted;
            EXHAUSTED
        } else {
            message.as_str()
        };
        let errors = self
            .outcome
            .errors
            .checked_add(1)
            .ok_or_else(|| CrawlError::Arithmetic {
                detail: "school site error counter overflow".to_owned(),
            })?;
        let note = self.outcome.note.get_or_insert_with(String::new);
        note.try_reserve(message.len())
            .map_err(|_| CrawlError::Resource {
                resource: "school site diagnostic allocation",
                requested,
                limit: MAX_DIAGNOSTIC_BYTES,
            })?;
        note.push_str(message);
        self.outcome.errors = errors;
        Ok(())
    }
}

fn merge(retained: &mut Signals, mut page: Signals) -> CrawlResult<()> {
    reserve(
        &mut retained.school_mailboxes,
        &page.school_mailboxes,
        16_384,
    )?;
    reserve(&mut retained.staff_contacts, &page.staff_contacts, 16_384)?;
    reserve(&mut retained.staff_pages, &page.staff_pages, 13)?;
    reserve(&mut retained.emails, &page.emails, 16_384)?;
    reserve(&mut retained.coach_hits, &page.coach_hits, 16_384)?;
    reserve(&mut retained.ad_hits, &page.ad_hits, 16_384)?;
    reserve(&mut retained.pages, &page.pages, 14)?;
    retained.school_mailboxes.append(&mut page.school_mailboxes);
    retained.staff_contacts.append(&mut page.staff_contacts);
    retained.staff_pages.append(&mut page.staff_pages);
    retained.emails.append(&mut page.emails);
    retained.coach_hits.append(&mut page.coach_hits);
    retained.ad_hits.append(&mut page.ad_hits);
    retained.pages.append(&mut page.pages);
    Ok(())
}

fn reserve<T>(retained: &mut Vec<T>, incoming: &[T], limit: usize) -> CrawlResult<()> {
    let requested =
        retained
            .len()
            .checked_add(incoming.len())
            .ok_or_else(|| CrawlError::Arithmetic {
                detail: "school site retained signal count overflow".to_owned(),
            })?;
    if requested > limit {
        return Err(CrawlError::Resource {
            resource: "school site retained signals",
            requested,
            limit,
        });
    }
    retained
        .try_reserve(incoming.len())
        .map_err(|_| CrawlError::Resource {
            resource: "school site signal allocation",
            requested,
            limit,
        })
}

#[cfg(test)]
mod tests {
    use super::merge;
    use crate::school_sites::{SchoolMailboxHit, Signals};
    use census_domain::model::SchoolMailboxPurpose;

    #[test]
    fn rejected_page_merge_does_not_append_an_earlier_candidate_prefix(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut retained = Signals {
            staff_pages: vec!["https://school.example/staff".to_owned(); 13],
            ..Signals::default()
        };
        let before = retained.clone();
        let mut page = Signals::default();
        page.school_mailboxes.push(SchoolMailboxHit {
            purpose: SchoolMailboxPurpose::SchoolOffice,
            email: "office@school.example".to_owned(),
            url: "https://school.example/new".to_owned(),
        });
        page.staff_pages
            .push("https://school.example/new".to_owned());
        check!(matches!(
            merge(&mut retained, page),
            Err(crate::CrawlError::Resource { .. })
        ));
        check!(eq; retained, before);
        Ok(())
    }
}
