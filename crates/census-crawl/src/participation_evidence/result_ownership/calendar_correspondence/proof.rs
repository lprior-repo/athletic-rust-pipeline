use super::objects::invalid;
use crate::participation_evidence::VerifiedIndependentParticipationV1;
use crate::CrawlResult;
use census_domain::result_calendar_correspondence::{
    RoncalliSeasonEventProofContextV1, RoncalliSeasonEventSelectionV1,
};

pub(super) fn preimage(
    source: &VerifiedIndependentParticipationV1,
    season: bool,
) -> CrawlResult<String> {
    if let Some(candidate) = source.roncalli_event() {
        let selection = RoncalliSeasonEventSelectionV1 {
            status_subject_start: candidate.status().subject_span.start,
            status_subject_end: candidate.status().subject_span.end,
            calendar_recap_id: candidate.event().recap_id.clone(),
        };
        if !selection.is_bounded() {
            return Err(invalid("calendar documentary selector exceeds bounds"));
        }
        return Ok(
            selection.interpretation_preimage(RoncalliSeasonEventProofContextV1 {
                status_body_sha: source.status_proof().capture(),
                schedule_body_sha: source.season_proof().capture(),
                status_locator: source.status_proof().locator(),
                calendar_locator: source.season_proof().locator(),
                day: source.day(),
            }),
        );
    }
    let name = match source.subject().document_namespace() {
        "mhsaa_article_v1" => "mhsaa_subject_dcc_event_day_v1",
        "milesplit_result_document_v1" => "milesplit_original_student_grade_dcc_event_day_v1",
        _ => return Err(invalid("unsupported documentary status namespace")),
    };
    let proof = if season {
        source.season_proof()
    } else {
        source.status_proof()
    };
    Ok(format!("{name}\n{}\n{}", proof.capture(), proof.locator()))
}
