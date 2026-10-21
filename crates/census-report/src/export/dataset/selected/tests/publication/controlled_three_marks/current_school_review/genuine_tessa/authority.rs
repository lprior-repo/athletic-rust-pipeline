use super::*;
use census_crawl::participation_evidence::recorded::{
    read_recorded_independent_participation, read_recorded_provider_school,
};
use census_crawl::participation_evidence::result_ownership::{
    calendar_correspondence as calendar, school_correspondence as school,
};
use census_domain::result_calendar_correspondence::*;
use census_domain::result_school_correspondence::*;
use census_store::{
    ResultCalendarCorrespondencePreparationV1, ResultCalendarEnrollmentReviewV1,
    ResultSchoolCorrespondencePreparationV1, ResultSchoolEnrollmentReviewV1,
};
mod calendar_review;
mod replay;
mod school_review;

pub(super) struct Accepted {
    school: (ResultSchoolReviewCommandV1, census_store::Receipt),
    calendar: (ResultCalendarReviewCommandV1, census_store::Receipt),
}

fn at() -> TestResult<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339("2026-10-21T22:10:00Z")?.to_utc())
}

pub(super) fn accept(installed: &source::Installed) -> TestResult<Accepted> {
    school_review::enroll(&installed.store)?;
    let school = school_review::accept(&installed.store, &school(installed)?)?;
    calendar_review::enroll(&installed.store)?;
    let calendar = calendar_review::accept(&installed.store, &calendar(installed)?)?;
    let accepted = Accepted { school, calendar };
    require_replay(&installed.store, &accepted)?;
    Ok(accepted)
}

pub(super) fn require_replay(store: &Store, accepted: &Accepted) -> TestResult {
    replay::school(store, accepted.school.0.clone(), &accepted.school.1)?;
    replay::calendar(store, accepted.calendar.0.clone(), &accepted.calendar.1)
}

fn school(installed: &source::Installed) -> TestResult<school::ResultSchoolSourceProposalV1> {
    let snapshot = installed.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let attachments =
        snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let provider =
        read_recorded_provider_school(&snapshot, &installed.school_reference, &mut grant)?
            .ok_or("retained genuine Roncalli school")?;
    let plan = discover_current_html_results(&attachments, &mut grant)?;
    check!(eq; plan.group_count(),1);
    check!(eq; plan.diagnostic_count(),0);
    let scope = school::ResultSchoolScopeV1 {
        day: ProfileDate::parse("2026-03-11")?,
        sport: Sport::IndoorTrack,
        category: Gender::Girls,
        year: SchoolYear::from_date("2026-03-11").ok_or("school year")?,
    };
    let proposal = plan.with_group(0, &mut grant, |linked, grant| {
        school::build_claim(&attachments, &linked, 0, &provider, scope, grant)
    })?;
    check!(eq; proposal.claim().statement.statement_text.as_str(),"Roncalli");
    check!(eq; proposal.claim().target.school,installed.school.id);
    check!(eq; proposal.claim().target.day.as_str(),"2026-03-11");
    Ok(proposal)
}

fn calendar(installed: &source::Installed) -> TestResult<calendar::ResultCalendarSourceProposalV1> {
    let snapshot = installed.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let attachments =
        snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let requested =
        read_recorded_independent_participation(&snapshot, &installed.participation, &mut grant)?
            .ok_or("genuine Roncalli selected pair")?;
    let plan = discover_current_html_results(&attachments, &mut grant)?;
    check!(eq; plan.group_count(),1);
    check!(eq; plan.diagnostic_count(),0);
    let proposal = plan.with_group(0, &mut grant, |linked, grant| {
        calendar::build_claim(&attachments, &linked, 0, &requested, grant)
    })?;
    check!(eq; proposal.claim().day.as_str(),"2026-03-11");
    check!(eq; proposal.claim().sport,Sport::IndoorTrack);
    check!(eq; proposal.claim().category,Gender::Girls);
    check!(eq; proposal.claim().year.get(),2025);
    check!(eq; proposal.claim().calendar.scope.as_str(),"https://athletics.roncalli.org/girls-track-field/schedules/#event-3387-recap");
    Ok(proposal)
}
