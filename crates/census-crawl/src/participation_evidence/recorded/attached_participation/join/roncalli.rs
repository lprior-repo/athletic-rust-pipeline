use super::*;
use crate::participation_evidence::result_ownership::{
    CheckedHtmlResultDocumentV1, CheckedHtmlResultRowV1,
};
use census_domain::model::SchoolYear;

pub(super) fn applicability(
    document: &CheckedHtmlResultDocumentV1<'_>,
    checked: &CheckedHtmlResultRowV1<'_>,
    source: &VerifiedIndependentParticipationV1,
) -> CrawlResult<Option<DocumentaryCohortAgreementV1>> {
    let (CheckedHtmlResultDocumentV1::Tfrrs(document), CheckedHtmlResultRowV1::Tfrrs(row)) =
        (document, checked)
    else {
        return Ok(None);
    };
    let candidate = source
        .roncalli_event()
        .ok_or_else(|| invalid("Roncalli checked event facts absent"))?;
    if row.published_date() != source.day().as_str() {
        return Ok(None);
    }
    if !document
        .capture()
        .requested_url()
        .starts_with("https://indiana.tfrrs.org/lists/")
        || source.subject().document_namespace() != "roncalli_girls_track_season_review_v1"
        || source.subject().school() != "Roncalli"
        || row.team_name() != "Roncalli"
        || row.team_path() != "/teams/tf/Roncalli_f.html"
        || document.sport() != Sport::IndoorTrack
        || row.gender() != Gender::Girls
    {
        return Err(invalid("Roncalli documentary TFRRS context differs").into());
    }
    let year = SchoolYear::from_date(row.published_date())
        .ok_or_else(|| invalid("Roncalli TFRRS canonical school year unsupported"))?;
    if source.subject().name() != row.athlete_name()
        || candidate.status().name != row.athlete_name()
        || candidate.status().grade != row.grade().get()
        || source.school_year() != year
        || document.year() != row.year()
        || i32::from(candidate.status().season_year) != i32::from(row.year())
        || candidate.event().day != *source.day()
        || GradYear::of(row.grade(), year) != Some(source.cohort())
    {
        return Err(invalid("Roncalli documentary TFRRS person day or grade differs").into());
    }
    let original = match row.original_mark() {
        Some(mark) => mark,
        None => row.published_mark(),
    };
    if row.event_label().strip_suffix(" Meters") != Some(candidate.event().event_label.as_str())
        || original != candidate.event().original_mark
    {
        return Err(invalid("Roncalli documentary TFRRS original event or mark differs").into());
    }
    Ok(Some(DocumentaryCohortAgreementV1::Agreed))
}

pub(super) fn calendar_requirement() -> DocumentarySeasonApplicabilityV1 {
    DocumentarySeasonApplicabilityV1::ReviewedCalendarRequired
}

#[cfg(test)]
mod tests;
