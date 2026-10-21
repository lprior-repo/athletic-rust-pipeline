use super::*;
use census_crawl::milesplit::LocatedRosterAthlete;
use census_crawl::participation_evidence::result_ownership::tfrrs::CheckedRetainedTfrrsRowV1;
use census_crawl::participation_evidence::VerifiedIndependentParticipationV1;

type CaptureDigests = (AttachmentDigestV1, AttachmentDigestV1);

pub(super) fn verify(
    installed: &source::Installed,
    target: &LocatedRosterAthlete,
    participation: &VerifiedIndependentParticipationV1,
    result: &CheckedRetainedTfrrsRowV1<'_>,
) -> TestResult {
    let current = target.athlete();
    let candidate = participation
        .roncalli_event()
        .ok_or("checked independent school person facts")?;
    check!(eq; current.name,installed.athlete.canonical_name);
    check!(eq; current.name,candidate.status().name);
    check!(eq; current.name,result.athlete_name());
    check!(eq; current.gender,Gender::Girls);
    check!(eq; current.gender,result.gender());
    check!(eq; current.grad_year,GradYear::CO2027);
    check!(eq; current.grad_year,participation.cohort());
    check!(eq; candidate.status().grade,result.grade().get());
    check!(eq; candidate.status().grade,11);
    check!(eq; candidate.status().season_year,candidate.event().season_year);
    check!(eq; i16::try_from(candidate.event().season_year)?,result.year());
    check!(eq; candidate.event().day.as_str(),result.published_date());
    check!(eq; result.team_name(),installed.school.name);
    check!(eq; result.athlete_id(),"8779607");
    check!(eq; result.original_mark(),Some(candidate.event().original_mark.as_str()));
    check!(eq; candidate.event().event_label,"3200");
    check!(eq; result.event_label(),"3200 Meters");
    check!(eq; result.meet_id(),"94245");
    check!(eq; result.performance_id(),"5841286");
    Ok(())
}

pub(super) fn context(
    objects: &mut Objects,
    target: &LocatedRosterAthlete,
    participation: &VerifiedIndependentParticipationV1,
    result: &CheckedRetainedTfrrsRowV1<'_>,
    captures: (&CaptureDigests, &CaptureDigests, &CaptureDigests),
) -> TestResult<AttachmentDigestV1> {
    let candidate = participation
        .roncalli_event()
        .ok_or("checked context facts")?;
    objects.json(&serde_json::json!({
        "schema":"genuine_tessa_ms_target_independent_roncalli_event_v1",
        "target_native_id":target.athlete().athlete_id,"target_row":target.locator(),
        "captures":{"roster":captures.0,"status":captures.1,"calendar":captures.2},
        "school_subject":participation.subject().reference(),
        "status_subject_span":candidate.status().subject_span,"status_statement_span":candidate.status().statement_span,
        "name":candidate.status().name,"grade":candidate.status().grade,"season_year":candidate.status().season_year,
        "day":candidate.event().day,"event":candidate.event().event_label,"original_mark":candidate.event().original_mark,
        "calendar_recap_id":candidate.event().recap_id,"calendar_recap_span":candidate.event().recap_span,
        "school_venue_label":candidate.event().venue,"school_finish_place":candidate.event().place,
        "tfrrs_native_id":result.athlete_id(),"tfrrs_row":result.locator(),"tfrrs_reference":result.reference(),
        "tfrrs_source_meet":result.meet_id(),"tfrrs_source_performance":result.performance_id(),
        "basis":"independent exact-person school event context reviewed for this document attachment only; no global same-person merge, provider finish place, venue equivalence or tenure inferred"
    }))
}
