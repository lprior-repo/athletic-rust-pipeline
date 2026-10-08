use super::{PageFailure, SchoolPages};
use crate::net::{FetchError, FetchOutcome, Fetcher};
use crate::ohsaa::collect::write::{emit_school, persist_failure, Projection};
use crate::ohsaa::collect::Tally;
use crate::ohsaa::{school_entities, SearchResult};
use crate::{AdapterContext, AdapterReport};
use census_domain::model::{CanonicalCoach, CoachRole, SchoolYear};
use census_store::{Store, Table};
use std::collections::HashMap;
use std::time::Duration;

struct Harness {
    _root: tempfile::TempDir,
    store: Store,
    fetcher: Fetcher,
}

impl Harness {
    fn new() -> anyhow::Result<Self> {
        let root = tempfile::tempdir()?;
        let store = Store::open(root.path().join("store"))?;
        let fetcher = Fetcher::new(
            root.path().join("http"),
            None,
            Duration::from_millis(1),
            HashMap::new(),
            vec![],
        )?
        .with_offline(true);
        Ok(Self {
            _root: root,
            store,
            fetcher,
        })
    }

    fn fail(&self, page: &str, failure: PageFailure) -> anyhow::Result<(AdapterReport, Tally)> {
        let ctx = AdapterContext {
            fetcher: &self.fetcher,
            store: &self.store,
            refresh: false,
            school_year: SchoolYear::new(2026).ok_or_else(|| anyhow::anyhow!("school year"))?,
            observed_on: "2026-10-02".to_string(),
            performance_as_of: chrono::NaiveDate::parse_from_str("2026-10-02", "%Y-%m-%d")?,
            recording: None,
        };
        let sr = dublin();
        let mut report = AdapterReport::new("ohsaa", "schools");
        let mut tally = Tally::default();
        failure.report(&sr, page, &mut report, &mut tally)?;
        if page == "sports" {
            persist_failure(&ctx, &sr, page, &failure, &ctx.observed_on)?;
            return Ok((report, tally));
        }
        let pages = SchoolPages {
            sports: sports_capture(),
            ad: Err(failure),
        };
        let mut extract = school_entities(&sr, &pages.sports, None);
        emit_school(
            &ctx,
            Projection {
                school: &sr,
                pages: &pages,
                extract: &mut extract,
            },
            &ctx.observed_on,
            &mut report,
            &mut tally,
        )?;
        report.with_email = tally.with_email;
        Ok((report, tally))
    }

    fn outcome(&self) -> anyhow::Result<serde_json::Value> {
        self.store
            .journal_payloads("ohsaa_school_outcomes_v1")?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("outcome absent"))
    }
}

fn dublin() -> SearchResult {
    SearchResult {
        name: "DUBLIN COFFMAN".to_string(),
        city: "Dublin".to_string(),
        ohsaa_id: "474".to_string(),
    }
}

fn sports_capture() -> FetchOutcome {
    let body = include_bytes!("../../../../tests/fixtures/ohsaa/sports_dublin_coffman.html");
    FetchOutcome {
        url: dublin().sports_url(),
        response_url: None,
        method: "GET".to_string(),
        status: 200,
        content_digest: crate::net::cache::content_digest(body),
        bytes: body.len(),
        fetched_at: "2026-09-01T10:00:00Z".to_string(),
        from_cache: true,
        content_type: Some("text/html".to_string()),
        body: body.to_vec(),
    }
}

fn http_failure(status: u16, url: String) -> anyhow::Result<PageFailure> {
    let capture = FetchOutcome {
        url,
        status,
        ..sports_capture()
    };
    Ok(PageFailure::Http(capture))
}

#[test]
fn failed_ad_http_outcomes_keep_valid_sports_and_never_stamp_school_success() -> anyhow::Result<()>
{
    for status in [404, 403, 500] {
        let run = Harness::new()?;
        let failure = http_failure(status, dublin().ad_url())?;
        let (report, tally) = run.fail("AD", failure)?;
        check!(eq; (report.rows, report.errors, report.with_email), (1, 1, 4));
        check!(eq; tally.fetch_failures, 1);
        check!(eq; tally.not_found, usize::from(status == 404));
        check!(eq; run.store.walk_table(Table::Schools)?.rows, 1);
        check!(eq; run.store.walk_table(Table::Coaches)?.rows, 4);
        check!(run.store.journal_keys("ohsaa_schools")?.is_empty());
        let coaches: Vec<CanonicalCoach> = run.store.scan(Table::Coaches)?;
        check!(coaches
            .iter()
            .all(|coach| coach.role == CoachRole::HeadCoach));
        let outcome = run.outcome()?;
        check!(eq; outcome["state"], "partial");
        check!(eq; outcome["pending_pages"], serde_json::json!([dublin().ad_url()]));
        check!(eq; outcome["failure"]["kind"], "http");
        check!(eq; outcome["failure"]["status"], status);
        check!(eq;
            outcome["failure"]["capture"]["capture_url"],
            dublin().ad_url()
        );
    }
    Ok(())
}

#[test]
fn sports_http_failures_are_counted_and_leave_both_page_obligations_unfinished(
) -> anyhow::Result<()> {
    for status in [404, 500] {
        let run = Harness::new()?;
        let (report, tally) = run.fail("sports", http_failure(status, dublin().sports_url())?)?;
        check!(eq; (report.rows, report.errors, report.with_email), (0, 1, 0));
        check!(eq; tally.fetch_failures, 1);
        check!(eq; tally.not_found, usize::from(status == 404));
        check!(eq; run.store.walk_table(Table::Schools)?.rows, 0);
        check!(eq; run.store.walk_table(Table::Coaches)?.rows, 0);
        check!(run.store.journal_keys("ohsaa_schools")?.is_empty());
        let outcome = run.outcome()?;
        check!(eq; outcome["state"], "failed");
        check!(eq;
            outcome["pending_pages"],
            serde_json::json!([dublin().sports_url(), dublin().ad_url()])
        );
        check!(eq; outcome["failure"]["status"], status);
    }
    Ok(())
}

#[test]
fn ad_transport_timeout_remains_retryable_partial_work() -> anyhow::Result<()> {
    let run = Harness::new()?;
    let failure = PageFailure::Fetch(FetchError::Timeout {
        url: dublin().ad_url(),
        timeout_secs: 45,
    });
    let (report, tally) = run.fail("AD", failure)?;
    check!(eq; (report.rows, report.errors, report.with_email), (1, 1, 4));
    check!(eq; (tally.fetch_failures, tally.not_found), (1, 0));
    check!(eq; run.store.walk_table(Table::Coaches)?.rows, 4);
    check!(run.store.journal_keys("ohsaa_schools")?.is_empty());
    let outcome = run.outcome()?;
    check!(eq; outcome["state"], "partial");
    check!(eq; outcome["failure"]["kind"], "fetch");
    check!(eq; outcome["failure"]["retryable"], true);
    Ok(())
}
