use super::super::{artifacts::SiteRecord, execution::State, research, QueueStats};
use census_crawl::school_sites::{PageEvidence, SchoolMailboxHit, Signals, SiteOutcome};
use census_domain::model::SchoolMailboxPurpose;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const URL: &str = "https://school.example/contact";

fn outcome() -> SiteOutcome {
    let mut signals = Signals::default();
    signals.school_mailboxes.push(SchoolMailboxHit {
        purpose: SchoolMailboxPurpose::SchoolOffice,
        email: "office@school.example".to_owned(),
        url: URL.to_owned(),
    });
    SiteOutcome {
        signals,
        requests: 1,
        errors: 0,
        note: None,
        pages: vec![PageEvidence {
            url: URL.to_owned(),
            digest: "a".repeat(64),
            fetched_at: "2026-10-07T12:34:56Z".to_owned(),
            status: 200,
        }],
    }
}

#[test]
fn isolated_candidates_retain_capture_without_claiming_verified_contacts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let site = super::site(census_domain::UsJurisdiction::Michigan, "Example", URL);
    let mut state = State::new(dir.path().to_owned(), QueueStats::default(), 1)?;
    state.missing_site(&site, &outcome())?;
    let report = state.finish()?;
    check!(eq; (report.crawled, report.empty, report.failed), (1, 0, 0));
    check!(eq; (report.emails, report.coach_contacts, report.ad_contacts), (0, 0, 0));
    let record: SiteRecord =
        serde_json::from_slice(&std::fs::read(dir.path().join("MI__example.json"))?)?;
    check!(eq; record.research_status, research::ResearchStatus::UnverifiedCandidates);
    check!(eq; record.coach_research_status, research::CoachResearchStatus::Unsupported);
    let candidate = record
        .school_mailbox_candidates
        .first()
        .ok_or("candidate")?;
    check!(eq; candidate.mailbox.as_str(), "office@school.example");
    check!(eq; candidate.source.url.as_str(), URL);
    check!(eq; candidate.source.fetched_at.as_str(), "2026-10-07T12:34:56Z");
    check!(eq; candidate.source.digest, "a".repeat(64));
    Ok(())
}

#[test]
fn absent_or_foreign_capture_never_supplies_candidate_provenance() -> TestResult {
    let mut outcome = outcome();
    outcome.pages.first_mut().ok_or("page")?.url = "https://other.example/".to_owned();
    let (candidates, status) = research::project(&outcome)?;
    check!(
        candidates.is_empty(),
        "foreign capture cannot qualify a candidate"
    );
    check!(eq; status, research::ResearchStatus::Incomplete);
    outcome.pages.clear();
    let (candidates, status) = research::project(&outcome)?;
    check!(
        candidates.is_empty(),
        "absent capture cannot qualify a candidate"
    );
    check!(eq; status, research::ResearchStatus::Incomplete);
    Ok(())
}

#[test]
fn partial_research_retains_supported_candidate_but_not_completion() -> TestResult {
    let mut outcome = outcome();
    outcome.errors = 1;
    outcome.note = Some("robots policy unavailable".to_owned());
    let (candidates, status) = research::project(&outcome)?;
    check!(eq; candidates.len(), 1);
    check!(eq; status, research::ResearchStatus::Incomplete);
    Ok(())
}

#[test]
fn empty_inspection_is_narrower_than_no_public_email_exists() -> TestResult {
    let mut outcome = outcome();
    outcome.signals.school_mailboxes.clear();
    let (candidates, status) = research::project(&outcome)?;
    check!(
        candidates.is_empty(),
        "inspection found no qualified candidate"
    );
    check!(eq; status, research::ResearchStatus::NoQualifiedCandidateInInspectedPages);
    Ok(())
}

#[test]
fn historical_site_artifacts_decode_with_unknown_research() -> TestResult {
    let record: SiteRecord = serde_json::from_value(serde_json::json!({
        "state": "MI", "school": "Example", "website": URL, "pages": [],
        "page_evidence": [], "requests": 1, "errors": 0
    }))?;
    check!(
        record.school_mailbox_candidates.is_empty(),
        "no invented historical candidates"
    );
    check!(eq; record.research_status, research::ResearchStatus::Unknown);
    check!(eq; record.coach_research_status, research::CoachResearchStatus::Unknown);
    Ok(())
}

#[test]
fn candidate_projection_refuses_excess_page_state() -> TestResult {
    let mut outcome = outcome();
    let page = outcome.pages.first().ok_or("page")?.clone();
    outcome.pages = vec![page; 15];
    check!(
        research::project(&outcome).is_err(),
        "page bound must refuse"
    );
    Ok(())
}

#[test]
fn conflicting_captures_at_one_url_do_not_guess_candidate_provenance() -> TestResult {
    let mut outcome = outcome();
    let mut page = outcome.pages.first().ok_or("page")?.clone();
    page.digest = "b".repeat(64);
    outcome.pages.push(page);
    let (candidates, status) = research::project(&outcome)?;
    check!(
        candidates.is_empty(),
        "capture identity must be unambiguous"
    );
    check!(eq; status, research::ResearchStatus::Incomplete);
    Ok(())
}

#[test]
fn unsuccessful_capture_is_not_a_completed_empty_inspection() -> TestResult {
    let mut outcome = outcome();
    outcome.pages.first_mut().ok_or("page")?.status = 403;
    let (candidates, status) = research::project(&outcome)?;
    check!(
        candidates.is_empty(),
        "blocked capture cannot support a candidate"
    );
    check!(eq; status, research::ResearchStatus::Incomplete);
    Ok(())
}
