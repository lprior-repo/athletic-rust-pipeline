use super::super::map::{
    directory_school, process_owned_summary, Capture, CoachEmission, DirectoryAdmission,
    SummaryEmission,
};
use super::super::parse::{parse_summary, DirectorySchool};
use super::super::summary_url;
use super::Run;
use crate::CrawlResult;
use census_domain::model::{CanonicalSchool, SchoolId};
use census_domain::UsJurisdiction;
use census_store::Table;

impl Run<'_> {
    pub(super) fn directory_owner(
        &mut self,
        state: UsJurisdiction,
        capture: Capture<'_>,
        row: &DirectorySchool,
    ) -> Option<String> {
        let Some(short_code) = row.short_code.as_deref().and_then(super::super::nonempty) else {
            self.reject(format!(
                "directory row {}: no short code to fetch a summary with",
                capture.url
            ));
            self.dropped_school_rows = self.dropped_school_rows.saturating_add(1);
            return None;
        };
        let (published, disposition) =
            match row.state_code.as_deref().and_then(UsJurisdiction::parse) {
                Some(published) if published == state => return Some(short_code),
                Some(published) => (Some(published), "foreign_published_state"),
                None if row.state_code.is_some() => (None, "unrecognized_published_state"),
                None => (None, "missing_or_unusable_published_state"),
            };
        self.reject(format!(
            "directory jurisdiction rejection {}: school {short_code}; disposition={disposition}; requested={}; published={}; observed_on={}; capture_sha256={}",
            capture.url,
            state.code(),
            published.map_or("unknown", |published| published.code()),
            capture.observed_on,
            capture.sha256
        ));
        None
    }

    pub(super) fn directory_page_matches(
        &mut self,
        parsed: &super::DirectoryPage,
        requested: usize,
        url: &str,
    ) -> bool {
        if parsed.current_page != requested {
            self.fail(format!(
                "directory page {url}: requested page {requested}, received {}",
                parsed.current_page
            ));
            return false;
        }
        if parsed.total_pages == 0 {
            self.fail(format!("directory page {url}: missing or zero totalPages"));
        }
        if parsed.total_results < parsed.results.len() {
            self.fail(format!(
                "directory page {url}: totalResults is below its published row count"
            ));
        }
        true
    }

    pub(super) fn admit_directory_school(
        &mut self,
        state: UsJurisdiction,
        association: &str,
        capture: Capture<'_>,
        row: &DirectorySchool,
    ) -> CrawlResult<Option<(Box<CanonicalSchool>, SchoolId)>> {
        match directory_school(state, association, row, capture.url, capture.observed_on)? {
            DirectoryAdmission::School(school, id) => Ok(Some((school, id))),
            DirectoryAdmission::MissingShortCode => {
                self.reject(format!(
                    "directory row {}: no short code to fetch a summary with",
                    capture.url
                ));
                self.dropped_school_rows = self.dropped_school_rows.saturating_add(1);
                Ok(None)
            }
            DirectoryAdmission::DroppedName => {
                self.dropped_school_rows = self.dropped_school_rows.saturating_add(1);
                Ok(None)
            }
        }
    }

    pub(super) fn retain_incomplete_summary(
        &mut self,
        school: &CanonicalSchool,
        emission: &CoachEmission,
    ) -> CrawlResult<()> {
        let digest = census_domain::model::serialized_digest(&(school, &emission.coaches))
            .map_err(|source| crate::CrawlError::Canonical {
                table: "school contact prefix".to_owned(),
                source,
            })?;
        let operation = format!("coach_directories_prefix_v1:{digest}");
        if self.ctx.effect_is_committed(&operation, &digest)? {
            return Ok(());
        }
        let mut batch = self.school_batch(school)?;
        batch.append_many(Table::Coaches, &emission.coaches)?;
        batch.commit_once(&operation, &digest)?;
        self.coach_rows = self.coach_rows.saturating_add(emission.coaches.len());
        let with_email = emission
            .coaches
            .iter()
            .filter(|coach| coach.has_published_email())
            .count();
        self.with_email = self
            .with_email
            .saturating_add(u64::try_from(with_email).map_or(u64::MAX, |count| count));
        Ok(())
    }

    pub(super) async fn fetch_and_process_summary(
        &mut self,
        row: &DirectorySchool,
        short_code: &str,
        school: &mut CanonicalSchool,
        school_id: &SchoolId,
    ) -> Option<SummaryEmission> {
        let url = summary_url(short_code);
        let outcome = self.summary_capture(&url, school).await?;
        let summary = self.parsed_summary(&outcome, school)?;
        let capture = Capture {
            url: outcome
                .response_url
                .as_deref()
                .map_or(outcome.url.as_str(), |url| url),
            observed_on: &outcome.fetched_at,
            sha256: &outcome.content_digest,
        };
        let mapped = process_owned_summary(
            school,
            row,
            &summary,
            school_id,
            capture,
            self.ctx.school_year,
        )
        .map_err(|error| match error {
            super::super::map::SummaryError::Coaches(error) => error,
            error => crate::CrawlError::Schema {
                url: capture.url.to_owned(),
                detail: error.to_string(),
            },
        });
        self.finish_summary(school, &outcome, mapped)
    }

    fn finish_summary(
        &mut self,
        school: &mut CanonicalSchool,
        outcome: &crate::net::FetchOutcome,
        mapped: CrawlResult<SummaryEmission>,
    ) -> Option<SummaryEmission> {
        match mapped {
            Ok(mapped) => {
                super::research::completed(school, self.ctx.school_year, outcome, &mapped.emission);
                if let Some(review) = &mapped.postal_review {
                    self.reject(format!("summary postal review {}: {review}", outcome.url));
                }
                Some(mapped)
            }
            Err(error) => {
                let attempt = super::research::attempt(
                    outcome,
                    super::super::research_failure::crawl(&error),
                    error.to_string(),
                );
                super::research::retain(school, self.ctx.school_year, attempt);
                self.fail(format!("summary association {}: {error}", outcome.url));
                None
            }
        }
    }

    async fn summary_capture(
        &mut self,
        url: &str,
        school: &mut CanonicalSchool,
    ) -> Option<crate::net::FetchOutcome> {
        match self.ctx.fetcher.get(url, &self.fetch).await {
            Ok(outcome) => Some(outcome),
            Err(error) => {
                let attempt = census_domain::model::ContactResearchAttempt {
                    locator: url.to_owned(),
                    acquired_at: crate::net::now_iso8601(),
                    source_sha256: None,
                    outcome: super::super::research_failure::fetch(&error),
                    reason: error.to_string(),
                };
                super::research::retain(school, self.ctx.school_year, attempt);
                self.fail(format!("fetch {url}: {error}"));
                None
            }
        }
    }

    fn parsed_summary(
        &mut self,
        outcome: &crate::net::FetchOutcome,
        school: &mut CanonicalSchool,
    ) -> Option<super::super::SchoolSummary> {
        match parse_summary(&outcome.body) {
            Ok(summary) => Some(summary),
            Err(error) => {
                let attempt = super::research::attempt(
                    outcome,
                    super::super::research_failure::crawl(&error),
                    error.to_string(),
                );
                super::research::retain(school, self.ctx.school_year, attempt);
                self.fail(format!("summary {}: {error}", outcome.url));
                None
            }
        }
    }
}
