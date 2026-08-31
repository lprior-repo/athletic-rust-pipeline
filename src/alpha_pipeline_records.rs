use crate::alpha_cohort::{classify_cohort, CohortDecision};
use crate::alpha_model::{RunUnit, SourceAthlete};
use crate::alpha_normalize::normalize_record;
use crate::alpha_output::{CohortException, UnresolvedRecord};
use crate::alpha_url::{canonical_state, validate_profile_url};
use std::collections::BTreeMap;

pub(crate) fn source_athlete(
    record: &crate::alpha_model::RankingRecord,
    unit: &RunUnit,
) -> Result<(SourceAthlete, Option<CohortException>), String> {
    if record.season_id != unit.season_id {
        return Err("ranking record season does not match requested unit".to_owned());
    }
    if !record.event_short.eq_ignore_ascii_case(&unit.event.event_short) {
        return Err("ranking record event does not match requested unit".to_owned());
    }
    let state = canonical_state(&record.state)
        .ok_or_else(|| "ranking record has invalid state evidence".to_owned())?;
    if state != unit.state.code {
        return Err("ranking record state does not match requested unit".to_owned());
    }
    let profile_url = if record.athlete_id > 0 {
        validate_profile_url(&format!("https://athletic.net/athlete/{}", record.athlete_id))
            .ok_or_else(|| "invalid confirmed athlete profile ID".to_owned())?
    } else {
        String::new()
    };
    let season_label = format!("{}-{:02}", record.season_id, (record.season_id + 1) % 100);
    let grade = i32::try_from(record.grade_id).ok();
    let decision = classify_cohort(2027, None, Some(&season_label), grade);
    let date = record
        .result_date
        .get(..10)
        .map_or_else(|| record.result_date.clone(), str::to_owned);
    let mut fields = BTreeMap::new();
    fields.insert("athlete_id".to_owned(), record.athlete_id.to_string());
    fields.insert("athlete_name".to_owned(), record.athlete_name.clone());
    fields.insert("school".to_owned(), record.team_name.clone());
    fields.insert("team_name".to_owned(), record.team_name.clone());
    fields.insert("state".to_owned(), state);
    fields.insert("grade_id".to_owned(), record.grade_id.to_string());
    fields.insert("gender".to_owned(), unit.gender.clone());
    fields.insert("sport".to_owned(), "Track and Field".to_owned());
    fields.insert("profile_url".to_owned(), profile_url.clone());
    fields.insert("source_url".to_owned(), profile_url.clone());
    fields.insert(
        "marks".to_owned(),
        format!(
            "{}|{}|{}|{}|{}|{}",
            record.event_short,
            record.measure,
            season_label,
            date,
            record.meet_name,
            record.wind.clone().map_or_else(String::new, |wind| wind)
        ),
    );
    fields.insert(
        "result_ids".to_owned(),
        record.result_id.map_or_else(String::new, |id| id.to_string()),
    );
    let source = crate::model::SourceRecord {
        source_key: "authorized-ranking".to_owned(),
        sheet: "alpha".to_owned(),
        excel_row: 0,
        fields,
    };
    let mut athlete = normalize_record(&source);
    athlete.cohort_evidence = decision.message().to_owned();
    let exception = match decision {
        CohortDecision::Exception(reason) | CohortDecision::Exclude(reason) => {
            Some(CohortException {
                athlete_id: record.athlete_id,
                reason,
                source_url: profile_url,
            })
        }
        CohortDecision::Include(_) => None,
    };
    Ok((athlete, exception))
}

pub(crate) fn unresolved_from(athlete: &SourceAthlete) -> UnresolvedRecord {
    let reason = match athlete.exception_notes.first() {
        Some(note) => note.clone(),
        None => "athlete ID missing".to_owned(),
    };
    let source_url = match athlete.source_urls.first() {
        Some(url) => url.clone(),
        None => String::new(),
    };
    UnresolvedRecord {
        record_key: "athlete-id-missing".to_owned(),
        reason,
        source_url,
    }
}
