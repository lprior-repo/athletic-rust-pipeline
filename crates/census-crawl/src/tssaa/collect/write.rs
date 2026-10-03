use census_domain::model::SourceNamespace;
use census_store::Table;
use serde_json::json;

use crate::net::FetchOutcome;
use crate::{school_observations_of, CrawlResult};

use super::super::{map::Emission, SOURCE_ID};
use super::{Run, JOURNAL};

impl Run<'_> {
    pub(super) fn emit(
        &mut self,
        key: &str,
        capture: &FetchOutcome,
        emission: Emission,
    ) -> CrawlResult<()> {
        let schools = std::slice::from_ref(&emission.school);
        let observations = school_observations_of(
            &SourceNamespace::association_school(SOURCE_ID),
            schools,
            &capture.fetched_at,
        );
        let emails = emission
            .coaches
            .iter()
            .filter(|coach| coach.has_published_email())
            .fold(0_u64, |count, _| count.saturating_add(1));
        let coaches = emission
            .coaches
            .iter()
            .fold(0_u64, |count, _| count.saturating_add(1));
        let mut batch = self.ctx.write_batch();
        batch.append_many(Table::Schools, schools)?;
        batch.append_many(Table::SourceObservations, &observations)?;
        batch.append_many(Table::Coaches, &emission.coaches)?;
        if emission.issues.is_empty() {
            batch.journal_done(
                JOURNAL,
                key,
                &self.completion_payload(capture, &emission, coaches, emails),
            )?;
        }
        batch.commit()?;
        if emission.issues.is_empty() {
            self.done.insert(key.to_string());
        }
        emission.issues.into_iter().for_each(|issue| {
            self.fail(format!(
                "incomplete school {}: {issue}; no completion marker written",
                capture.url
            ));
        });
        self.report.rows = self.report.rows.saturating_add(1);
        self.report.with_email = self.report.with_email.saturating_add(emails);
        self.coaches = self.coaches.saturating_add(coaches);
        Ok(())
    }

    fn completion_payload(
        &self,
        capture: &FetchOutcome,
        emission: &Emission,
        coaches: u64,
        emails: u64,
    ) -> serde_json::Value {
        let evaluated_on = if self.options.observed_on.trim().is_empty() {
            &self.ctx.observed_on
        } else {
            &self.options.observed_on
        };
        json!({
            "state": "TN",
            "school_id": emission.school.source_identities.first().map(|owner| &owner.id),
            "school": emission.school.name,
            "coach_rows": coaches,
            "with_email": emails,
            "capture_url": capture.url,
            "capture_sha256": capture.content_digest,
            "captured_at": capture.fetched_at,
            "evaluated_on": evaluated_on,
        })
    }
}
