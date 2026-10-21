use super::{HtmlResultProjectionError as Error, Result, INTERPRETATION};
use crate::milesplit::SourceRowLocator;
use census_domain::model::{CanonicalPerformance, Evidence, EvidenceMethod};
use serde::Deserialize;
use serde_json::value::RawValue;

pub fn matches_physical_observation(
    actual: &CanonicalPerformance,
    projected: &CanonicalPerformance,
) -> Result<bool> {
    if !physical_fields_match(actual, projected) {
        return Ok(false);
    }
    let baseline = projected
        .evidence
        .first()
        .ok_or(Error::Invalid("physical projection has no source evidence"))?;
    if actual.evidence.is_empty()
        || baseline.method != EvidenceMethod::Parsed
        || baseline.source.id != INTERPRETATION
        || baseline.source.url.as_deref().is_none_or(str::is_empty)
    {
        return Ok(false);
    }
    let expected = parse_evidence(baseline, &projected.source_key)?;
    for evidence in actual
        .evidence
        .iter()
        .chain(projected.evidence.iter().skip(1))
    {
        if evidence.source != baseline.source
            || evidence.method != baseline.method
            || evidence.observed_on != baseline.observed_on
        {
            return Ok(false);
        }
        let note = parse_evidence(evidence, &projected.source_key)?;
        if !immutable_note_matches(&note, &expected) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn physical_fields_match(actual: &CanonicalPerformance, projected: &CanonicalPerformance) -> bool {
    let CanonicalPerformance {
        id,
        athlete,
        team,
        event,
        meet,
        date,
        mark,
        wind_mps,
        place,
        heat,
        round,
        timing,
        observed_grade,
        adjustment,
        combined_completion,
        evidence: _,
        source_key,
        source_athlete,
        retained_conflicts,
    } = actual;
    id == &projected.id
        && athlete == &projected.athlete
        && team == &projected.team
        && event == &projected.event
        && meet == &projected.meet
        && date == &projected.date
        && mark == &projected.mark
        && wind_mps == &projected.wind_mps
        && place == &projected.place
        && heat == &projected.heat
        && round == &projected.round
        && timing == &projected.timing
        && observed_grade == &projected.observed_grade
        && adjustment == &projected.adjustment
        && combined_completion == &projected.combined_completion
        && source_key == &projected.source_key
        && source_athlete == &projected.source_athlete
        && retained_conflicts == &projected.retained_conflicts
}

fn parse_evidence<'a>(evidence: &'a Evidence, source_key: &str) -> Result<RowNote<'a>> {
    let note = evidence
        .note
        .as_deref()
        .ok_or(Error::Invalid("physical source evidence note absent"))?;
    if note.len() > 8192 {
        return Err(Error::Budget {
            resource: "physical source evidence note",
            needed: note.len(),
            limit: 8192,
        });
    }
    let parsed: RowNote<'_> = serde_json::from_str(note)
        .map_err(|_| Error::Invalid("physical source evidence note malformed"))?;
    if parsed.interpretation != INTERPRETATION
        || parsed.admission != "unadmitted_projection"
        || parsed.content_key != source_key
        || !parsed
            .proposed_target
            .proposed_attachment_reference
            .get()
            .starts_with('"')
        || !parsed
            .proposed_target
            .proposed_school_binding_reference
            .get()
            .starts_with('"')
    {
        return Err(Error::Invalid(
            "physical source evidence interpretation differs",
        ));
    }
    Ok(parsed)
}

fn immutable_note_matches(left: &RowNote<'_>, right: &RowNote<'_>) -> bool {
    left.context_reference == right.context_reference
        && left.original_row_reference == right.original_row_reference
        && left.original_row_digest == right.original_row_digest
        && left.proposed_target.locator == right.proposed_target.locator
        && left.proposed_target.subject_reference == right.proposed_target.subject_reference
        && left.proposed_target.proposed_athlete == right.proposed_target.proposed_athlete
        && left.proposed_target.proposed_school == right.proposed_target.proposed_school
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RowNote<'a> {
    interpretation: &'a str,
    admission: &'a str,
    content_key: &'a str,
    context_reference: &'a str,
    original_row_reference: &'a str,
    original_row_digest: &'a str,
    #[serde(borrow)]
    proposed_target: ProposedTarget<'a>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProposedTarget<'a> {
    locator: SourceRowLocator,
    subject_reference: &'a str,
    proposed_athlete: &'a str,
    proposed_school: &'a str,
    #[serde(borrow)]
    proposed_attachment_reference: &'a RawValue,
    #[serde(borrow)]
    proposed_school_binding_reference: &'a RawValue,
}
