use super::{counter_error, Projection, Run, JOURNAL, SOURCE_ID};
use crate::CrawlResult;
use census_domain::model::{Id, SourceNamespace};
use census_store::Table;

impl Run<'_> {
    pub(super) fn write(&mut self, projection: Projection<'_>) -> CrawlResult<()> {
        let key = projection_key(&projection);
        if let Some(capture) = &projection.detail {
            crate::coach_directories::persist_staff_capture(
                self.ctx,
                &projection.school,
                capture,
                &projection.coaches,
                projection.detail_outcome.clone(),
            )?;
        } else if projection.detail_outcome == super::Outcome::Unattempted {
            crate::coach_directories::persist_unattempted(self.ctx, &projection.school)?;
        } else {
            crate::coach_directories::persist_staff_attempt(
                self.ctx,
                &projection.school,
                census_domain::model::ContactResearchAttempt {
                    locator: projection
                        .school
                        .source_identities
                        .first()
                        .and_then(|owner| owner.url.clone())
                        .ok_or_else(|| crate::CrawlError::Invariant {
                            detail: "PIAA school has no details locator".to_owned(),
                        })?,
                    acquired_at: crate::net::now_iso8601(),
                    source_sha256: None,
                    outcome: projection.detail_outcome.clone(),
                    reason: "details acquisition failed".to_owned(),
                },
            )?;
        }
        let operation = format!("{JOURNAL}:{key}");
        if self
            .ctx
            .effect_is_committed(&operation, &projection.list.content_digest)?
        {
            return Ok(());
        }
        let mut batch = self.ctx.write_batch();
        batch.append_many(Table::Schools, std::slice::from_ref(&projection.school))?;
        batch.append_many(
            Table::SourceObservations,
            &crate::school_observations_of(
                &SourceNamespace::association_school(SOURCE_ID),
                std::slice::from_ref(&projection.school),
                &projection.list.fetched_at,
            ),
        )?;
        batch.append_many(Table::Coaches, &projection.coaches)?;
        batch.journal_done(JOURNAL, &key, &serde_json::json!({
            "school":projection.school.id,"list_url":projection.list.url,"list_sha256":projection.list.content_digest,
            "list_fetched_at":projection.list.fetched_at,
            "details":projection.detail.as_ref().map(|capture| serde_json::json!({"url":capture.url,"sha256":capture.content_digest,"fetched_at":capture.fetched_at})),
        }))?;
        batch.commit_once(&operation, &projection.list.content_digest)?;
        self.report.rows = self.report.rows.checked_add(1).ok_or_else(counter_error)?;
        let emails = u64::try_from(
            projection
                .coaches
                .iter()
                .filter(|coach| coach.has_published_email())
                .count(),
        )
        .map_err(|_| counter_error())?;
        self.report.with_email = self
            .report
            .with_email
            .checked_add(emails)
            .ok_or_else(counter_error)?;
        Ok(())
    }
}

fn projection_key(projection: &Projection<'_>) -> String {
    let detail = projection.detail.as_ref();
    Id::<()>::mint(
        JOURNAL,
        &[
            projection.school.id.as_str(),
            &projection.list.url,
            &projection.list.content_digest,
            &projection.list.fetched_at,
            detail.map_or("", |capture| capture.url.as_str()),
            detail.map_or("", |capture| capture.content_digest.as_str()),
            detail.map_or("", |capture| capture.fetched_at.as_str()),
        ],
    )
    .to_string()
}
