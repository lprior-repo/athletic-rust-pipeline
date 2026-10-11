use super::{Capture, DirectorySchool, Run};
use crate::coach_directories::map::{retain_directory_postal, SummaryEmission};
use crate::CrawlResult;
use census_domain::model::{CanonicalSchool, SchoolId};
use census_domain::UsJurisdiction;

struct PreparedSchool {
    school: CanonicalSchool,
    owner: SchoolId,
    key: String,
    short_code: String,
    postal_complete: bool,
}

impl Run<'_> {
    pub(super) async fn process_school(
        &mut self,
        state: UsJurisdiction,
        association: &str,
        capture: Capture<'_>,
        row: &DirectorySchool,
    ) -> CrawlResult<()> {
        let Some(mut prepared) = self.prepare_school(state, association, capture, row)? else {
            return Ok(());
        };
        let mapped = self
            .fetch_and_process_summary(
                row,
                &prepared.short_code,
                &mut prepared.school,
                &prepared.owner,
            )
            .await;
        super::super::generic::research_school_mailboxes(self.ctx, &mut prepared.school).await?;
        let Some(mapped) = mapped else {
            self.incomplete_states.insert(state);
            self.school_batch(&prepared.school)?.commit()?;
            return Ok(());
        };
        self.complete_school(state, prepared, mapped)
    }

    fn prepare_school(
        &mut self,
        state: UsJurisdiction,
        association: &str,
        capture: Capture<'_>,
        row: &DirectorySchool,
    ) -> CrawlResult<Option<PreparedSchool>> {
        let Some(short_code) = self.directory_owner(state, capture, row) else {
            return Ok(None);
        };
        let Some((mut school, owner)) =
            self.admit_directory_school(state, association, capture, row)?
        else {
            return Ok(None);
        };
        if !self.wanted.is_empty() && !self.wanted.contains(&school.normalized_name) {
            return Ok(None);
        }
        let key = format!(
            "{}:{}:{short_code}",
            self.ctx.school_year.short(),
            state.code()
        );
        if self.done.contains(&key) && !self.fetch.refresh {
            if !self.wanted.is_empty() {
                self.seen_names.insert(school.normalized_name);
            }
            self.skipped = self.skipped.saturating_add(1);
            return Ok(None);
        }
        let postal_complete = self.directory_postal(&mut school, row, capture);
        Ok(Some(PreparedSchool {
            school: *school,
            owner,
            key,
            short_code,
            postal_complete,
        }))
    }

    fn directory_postal(
        &mut self,
        school: &mut CanonicalSchool,
        row: &DirectorySchool,
        capture: Capture<'_>,
    ) -> bool {
        match retain_directory_postal(school, row, capture) {
            Ok(()) => true,
            Err(review) => {
                self.fail(format!("directory postal review {}: {review}", capture.url));
                false
            }
        }
    }

    fn complete_school(
        &mut self,
        state: UsJurisdiction,
        prepared: PreparedSchool,
        mapped: SummaryEmission,
    ) -> CrawlResult<()> {
        let contacts_complete = census_domain::model::ContactResearch::programs()
            .iter()
            .all(|program| {
                census_domain::model::school_contact_research(
                    &prepared.school,
                    program,
                    self.ctx.school_year,
                )
                .is_terminal()
            })
            && [
                census_domain::model::SchoolMailboxPurpose::SchoolOffice,
                census_domain::model::SchoolMailboxPurpose::AthleticsOffice,
            ]
            .into_iter()
            .all(|purpose| {
                census_domain::model::school_mailbox_research(
                    &prepared.school,
                    purpose,
                    self.ctx.school_year,
                )
                .is_terminal()
            });
        if !contacts_complete {
            self.incomplete_states.insert(state);
        }
        let emission = mapped.emission;
        if prepared.postal_complete && mapped.postal_review.is_none() && contacts_complete {
            self.write(
                &prepared.key,
                &prepared.school,
                &emission.coaches,
                &prepared.short_code,
            )?;
        } else {
            self.retain_incomplete_summary(&prepared.school, &emission)?;
        }
        self.counters.absorb(&emission.counters);
        self.processed = self.processed.saturating_add(1);
        self.remember_source(prepared);
        Ok(())
    }

    fn remember_source(&mut self, prepared: PreparedSchool) {
        if !self.wanted.is_empty() {
            self.seen_names.insert(prepared.school.normalized_name);
        }
        if let Some(attempt) = prepared
            .school
            .contact_research
            .into_iter()
            .next()
            .and_then(|row| row.attempts.into_iter().next())
        {
            self.researched.insert(prepared.owner, attempt.locator);
        }
    }
}
