use super::super::{
    artifacts::SiteRecord, execution::State, research::CoachResearchStatus, staff_research,
    QueueStats,
};
use census_crawl::school_sites::{PageEvidence, Signals, SiteOutcome, StaffContactHit};
use census_domain::model::{CoachRole, Gender, Sport};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const URL: &str = "https://school.example/staff-directory";

fn outcome() -> SiteOutcome {
    let mut signals = Signals::default();
    signals.staff_pages.push(URL.to_owned());
    signals.staff_contacts.push(StaffContactHit {
        published_school: "Example High School".to_owned(),
        member_id: "42".to_owned(),
        category_id: "xc".to_owned(),
        name: "Jane Doe".to_owned(),
        published_role: "Head Coach".to_owned(),
        role: CoachRole::HeadCoach,
        published_sport: "Girls Cross Country".to_owned(),
        sport: Some(Sport::CrossCountry),
        gender: Gender::Girls,
        published_level: "Varsity".to_owned(),
        email: "jdoe@school.example".to_owned(),
        url: URL.to_owned(),
    });
    SiteOutcome {
        signals,
        requests: 1,
        errors: 0,
        note: None,
        pages: vec![PageEvidence {
            url: URL.to_owned(),
            digest: "c".repeat(64),
            fetched_at: "2026-10-07T12:34:56Z".to_owned(),
            status: 200,
        }],
    }
}

#[test]
fn staff_candidate_artifact_retains_published_fields_without_accepting_contacts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let site = super::site(
        census_domain::UsJurisdiction::Michigan,
        "Queue Selector",
        URL,
    );
    let mut state = State::new(dir.path().to_owned(), QueueStats::default(), 1)?;
    state.missing_site(&site, &outcome())?;
    let report = state.finish()?;
    check!(eq; (report.crawled, report.empty, report.failed), (1, 0, 0));
    check!(eq; (report.emails, report.coach_contacts, report.ad_contacts), (0, 0, 0));
    let record: SiteRecord =
        serde_json::from_slice(&std::fs::read(dir.path().join("MI__queue-selector.json"))?)?;
    check!(eq; record.coach_research_status, CoachResearchStatus::UnverifiedCandidates);
    let candidate = record
        .staff_contact_candidates
        .first()
        .ok_or("staff candidate")?;
    let expected = outcome();
    check!(eq; Some(&candidate.staff), expected.signals.staff_contacts.first());
    check!(eq; candidate.source.url.as_str(), URL);
    check!(eq; candidate.source.fetched_at.as_str(), "2026-10-07T12:34:56Z");
    check!(eq; candidate.source.digest, "c".repeat(64));
    check!(eq; record.school.as_str(), "Queue Selector");
    Ok(())
}

#[test]
fn unsupported_markup_remains_distinct_from_recognized_empty_staff_page() -> TestResult {
    let mut outcome = outcome();
    outcome.signals.staff_contacts.clear();
    let (candidates, status) = staff_research::project(&outcome)?;
    check!(
        candidates.is_empty(),
        "recognized page contains no qualified candidates"
    );
    check!(eq; status, CoachResearchStatus::NoQualifiedCandidateInRecognizedStaffPages);
    outcome.signals.staff_pages.clear();
    let (candidates, status) = staff_research::project(&outcome)?;
    check!(
        candidates.is_empty(),
        "unsupported page cannot invent candidates"
    );
    check!(eq; status, CoachResearchStatus::Unsupported);
    Ok(())
}

#[test]
fn staff_candidate_requires_exact_unambiguous_successful_capture() -> TestResult {
    let mut outcome = outcome();
    outcome.pages.first_mut().ok_or("page")?.url = "https://other.example/staff".to_owned();
    let (candidates, status) = staff_research::project(&outcome)?;
    check!(
        candidates.is_empty(),
        "foreign capture cannot qualify staff candidate"
    );
    check!(eq; status, CoachResearchStatus::Incomplete);
    outcome.pages.first_mut().ok_or("page")?.url = URL.to_owned();
    let mut changed = outcome.pages.first().ok_or("page")?.clone();
    changed.digest = "d".repeat(64);
    outcome.pages.push(changed);
    let (candidates, status) = staff_research::project(&outcome)?;
    check!(
        candidates.is_empty(),
        "ambiguous capture cannot qualify staff candidate"
    );
    check!(eq; status, CoachResearchStatus::Incomplete);
    Ok(())
}

#[test]
fn unrecognized_candidate_page_cannot_claim_staff_parser_coverage() -> TestResult {
    let mut outcome = outcome();
    outcome.signals.staff_pages.clear();
    let (candidates, status) = staff_research::project(&outcome)?;
    check!(
        candidates.is_empty(),
        "candidate must come from recognized staff page"
    );
    check!(eq; status, CoachResearchStatus::Incomplete);
    Ok(())
}

#[test]
fn partial_staff_research_retains_candidates_without_claiming_completion() -> TestResult {
    let mut outcome = outcome();
    outcome.errors = 1;
    outcome.note = Some("next page blocked".to_owned());
    let (candidates, status) = staff_research::project(&outcome)?;
    check!(eq; candidates.len(), 1);
    check!(eq; status, CoachResearchStatus::Incomplete);
    Ok(())
}

#[test]
fn blocked_staff_capture_does_not_count_as_an_empty_inspection() -> TestResult {
    let mut outcome = outcome();
    outcome.signals.staff_contacts.clear();
    outcome.pages.first_mut().ok_or("page")?.status = 403;
    let (candidates, status) = staff_research::project(&outcome)?;
    check!(
        candidates.is_empty(),
        "blocked capture cannot produce staff contacts"
    );
    check!(eq; status, CoachResearchStatus::Incomplete);
    Ok(())
}

#[test]
fn historical_office_only_artifacts_keep_unsupported_coach_status() -> TestResult {
    let record: SiteRecord = serde_json::from_value(serde_json::json!({
        "state": "MI", "school": "Example", "website": URL, "pages": [],
        "page_evidence": [], "requests": 1, "errors": 0,
        "coach_research_status": "unsupported"
    }))?;
    check!(
        record.staff_contact_candidates.is_empty(),
        "no invented historical staff candidates"
    );
    check!(eq; record.coach_research_status, CoachResearchStatus::Unsupported);
    Ok(())
}

#[test]
fn staff_projection_enforces_recognized_page_budget() -> TestResult {
    let mut outcome = outcome();
    outcome.signals.staff_pages = vec![URL.to_owned(); 15];
    check!(
        staff_research::project(&outcome).is_err(),
        "recognized page budget must refuse"
    );
    Ok(())
}

#[test]
fn incomplete_staff_provenance_prevents_successful_empty_report() -> TestResult {
    let dir = tempfile::tempdir()?;
    let site = super::site(census_domain::UsJurisdiction::Michigan, "Example", URL);
    let mut outcome = outcome();
    outcome
        .signals
        .staff_pages
        .push("https://school.example/unretained".to_owned());
    let mut state = State::new(dir.path().to_owned(), QueueStats::default(), 1)?;
    state.missing_site(&site, &outcome)?;
    let report = state.finish()?;
    check!(eq; (report.crawled, report.empty, report.failed), (1, 0, 1));
    Ok(())
}

#[test]
fn athletic_director_candidate_preserves_its_non_team_role() -> TestResult {
    let mut outcome = outcome();
    let hit = outcome
        .signals
        .staff_contacts
        .first_mut()
        .ok_or("staff hit")?;
    hit.role = CoachRole::AthleticDirector;
    hit.published_role = "Athletic Director".to_owned();
    hit.sport = None;
    hit.published_sport = "Administration".to_owned();
    hit.gender = Gender::Unknown;
    let (candidates, status) = staff_research::project(&outcome)?;
    check!(eq; status, CoachResearchStatus::UnverifiedCandidates);
    let candidate = candidates.first().ok_or("director candidate")?;
    check!(eq; candidate.staff.role, CoachRole::AthleticDirector);
    check!(eq; candidate.staff.sport, None);
    check!(eq; candidate.staff.published_role.as_str(), "Athletic Director");
    check!(eq; candidate.staff.published_sport.as_str(), "Administration");
    Ok(())
}
