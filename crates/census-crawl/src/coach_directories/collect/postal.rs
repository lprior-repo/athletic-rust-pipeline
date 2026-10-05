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
        let mut batch = self.school_batch(school)?;
        batch.append_many(Table::Coaches, &emission.coaches)?;
        batch.commit()?;
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
        let outcome = self.get(&url).await?;
        let summary = match parse_summary(&outcome.body) {
            Ok(summary) => summary,
            Err(error) => {
                self.fail(format!("summary {url}: {error}"));
                return None;
            }
        };
        let capture = Capture {
            url: &outcome.url,
            observed_on: &outcome.fetched_at,
            sha256: &outcome.content_digest,
        };
        match process_owned_summary(
            school,
            row,
            &summary,
            school_id,
            capture,
            self.ctx.school_year,
        ) {
            Ok(mapped) => {
                if let Some(review) = &mapped.postal_review {
                    self.reject(format!("summary postal review {url}: {review}"));
                }
                Some(mapped)
            }
            Err(error) => {
                self.fail(format!("summary association {url}: {error}"));
                None
            }
        }
    }
}
