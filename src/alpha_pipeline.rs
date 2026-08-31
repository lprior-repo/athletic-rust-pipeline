use crate::alpha_api_client::AlphaApiClient;
use crate::alpha_catalog::parse_nav_targets;
use crate::alpha_checkpoint::{append, load_latest, AlphaCheckpoint, AlphaUnitKey};
use crate::alpha_cohort::{classify_cohort, CohortDecision};
use crate::alpha_config::AlphaConfig;
use crate::alpha_merge::merge_athlete;
use crate::alpha_model::{AlphaRequest, RunMatrix, RunUnit, SourceAthlete};
use crate::alpha_normalize::normalize_record;
use crate::alpha_output::{
    read_jsonl, write_outputs, CohortException, CoverageReport, UnresolvedRecord,
};
use crate::alpha_url::validate_profile_url;
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize)]
pub struct CollectionSummary {
    pub coverage: CoverageReport,
    pub athlete_count: usize,
    pub cohort_exception_count: usize,
    pub unresolved_count: usize,
}

pub async fn collect_authorized(
    config_path: &Path,
    output_dir: &Path,
    max_units: Option<usize>,
    authorization_ack: bool,
) -> Result<CollectionSummary> {
    let config = AlphaConfig::load(config_path)?;
    if !config.authorization.enabled {
        bail!("authorization.enabled must be true for collect-authorized");
    }
    if !authorization_ack {
        bail!("collect-authorized requires --i-have-alpha-authorization");
    }
    let client = AlphaApiClient::new(config.to_client_config())?;
    let nav = client
        .nav_info(config.authorization.allowed_seasons[0], false)
        .await
        .context("loading authorized alpha catalog")?;
    let (states, events) = parse_nav_targets(vec![nav]).map_err(|error| anyhow::anyhow!(error))?;
    let matrix = RunMatrix::from_targets(
        states,
        config.authorization.allowed_seasons.clone(),
        config.authorization.allowed_genders.clone(),
        events,
    )
    .map_err(|error| anyhow::anyhow!(error))?;
    let units = matrix.take(max_units).to_vec();
    let latest = load_latest(output_dir)?;
    let mut coverage = CoverageReport {
        planned_units: units.len(),
        ..CoverageReport::default()
    };
    let mut keyed = BTreeMap::new();
    for athlete in read_jsonl::<SourceAthlete>(&output_dir.join("athletes.jsonl"))? {
        if athlete.athlete_id > 0 {
            merge_athlete(&mut keyed, athlete);
        }
    }
    let mut cohort_exceptions =
        read_jsonl::<CohortException>(&output_dir.join("cohort-exceptions.jsonl"))?;
    let mut unresolved = read_jsonl::<UnresolvedRecord>(&output_dir.join("unresolved.jsonl"))?;

    for unit in units {
        let prior = latest.values().find(|state| same_unit(&state.key, &unit));
        if prior.is_some_and(|state| state.complete) {
            coverage.complete_units += 1;
            continue;
        }
        let mut continuation = match prior {
            Some(checkpoint) => checkpoint_continuation(checkpoint)
                .map_err(|error| anyhow::anyhow!(error))?,
            None => None,
        };
        loop {
            let key = unit_key(&unit, &continuation)
                .map_err(|error| anyhow::anyhow!(error))?;
            let page = client
                .rankings(&AlphaRequest {
                    state_id: unit.state.state_id,
                    season_id: unit.season_id,
                    gender: unit.gender.clone(),
                    event_short: unit.event.event_short.clone(),
                    indoor: false,
                    continuation: continuation.clone(),
                })
                .await;
            let page = match page {
                Ok(page) => page,
                Err(error) => {
                    append_checkpoint(output_dir, key, 0, false, "error")?;
                    return Err(anyhow::anyhow!(error));
                }
            };
            let response_count = page.records.len();
            for record in page.records {
                match source_athlete(&record, &unit) {
                    Ok((_athlete, Some(exception))) => cohort_exceptions.push(exception),
                    Ok((athlete, None)) if athlete.athlete_id == 0 => {
                        unresolved.push(unresolved_from(&athlete));
                    }
                    Ok((athlete, None)) if !athlete.exception_notes.is_empty() => {
                        unresolved.push(unresolved_from(&athlete));
                    }
                    Ok((athlete, None)) => {
                        merge_athlete(&mut keyed, athlete);
                    }
                    Err(reason) => unresolved.push(UnresolvedRecord {
                        record_key: "ranking-record".to_owned(),
                        reason,
                        source_url: String::new(),
                    }),
                }
            }
            if page.complete {
                append_checkpoint(output_dir, key, response_count, true, "complete")?;
                coverage.complete_units += 1;
                if response_count == 0 {
                    coverage.empty_units += 1;
                }
                break;
            }
            let Some(next) = page.continuation else {
                append_checkpoint(output_dir, key, response_count, false, "incomplete")?;
                bail!("authorized alpha unit is incomplete without continuation");
            };
            if continuation.as_ref() == Some(&next) {
                append_checkpoint(output_dir, key, response_count, false, "loop")?;
                bail!("authorized alpha continuation did not advance");
            }
            let next_key = unit_key(&unit, &Some(next.clone()))
                .map_err(|error| anyhow::anyhow!(error))?;
            append_checkpoint(output_dir, next_key, response_count, false, "incomplete")?;
            continuation = Some(next);
        }
    }
    let athletes: Vec<SourceAthlete> = keyed.into_values().collect();
    write_outputs(output_dir, &athletes, &cohort_exceptions, &unresolved, &coverage)?;
    Ok(CollectionSummary {
        coverage,
        athlete_count: athletes.len(),
        cohort_exception_count: cohort_exceptions.len(),
        unresolved_count: unresolved.len(),
    })
}

fn unit_key(unit: &RunUnit, continuation: &Option<serde_json::Value>) -> Result<AlphaUnitKey, String> {
    let continuation = match continuation {
        None => "0".to_owned(),
        Some(value) => serde_json::to_string(value).map_err(|_| "invalid continuation".to_owned())?,
    };
    Ok(AlphaUnitKey {
        state_code: unit.state.code.clone(),
        season_id: unit.season_id,
        gender: unit.gender.clone(),
        event_short: unit.event.event_short.clone(),
        continuation,
    })
}

fn same_unit(key: &AlphaUnitKey, unit: &RunUnit) -> bool {
    key.state_code == unit.state.code
        && key.season_id == unit.season_id
        && key.gender == unit.gender
        && key.event_short == unit.event.event_short
}

fn checkpoint_continuation(
    checkpoint: &AlphaCheckpoint,
) -> Result<Option<serde_json::Value>, String> {
    if checkpoint.key.continuation.is_empty() || checkpoint.key.continuation == "0" {
        return Ok(None);
    }
    serde_json::from_str(&checkpoint.key.continuation)
        .map(Some)
        .map_err(|_| "invalid checkpoint continuation".to_owned())
}

fn append_checkpoint(
    output_dir: &Path,
    key: AlphaUnitKey,
    response_count: usize,
    complete: bool,
    status: &str,
) -> Result<()> {
    append(output_dir, &AlphaCheckpoint {
        key,
        response_count,
        complete,
        status: status.to_owned(),
    })
}

fn source_athlete(
    record: &crate::alpha_model::RankingRecord,
    unit: &RunUnit,
) -> Result<(SourceAthlete, Option<CohortException>), String> {
    let profile_url = if record.athlete_id > 0 {
        validate_profile_url(&format!("https://athletic.net/athlete/{}", record.athlete_id))
            .ok_or_else(|| "invalid confirmed athlete profile ID".to_owned())?
    } else {
        String::new()
    };
    let season_label = format!("{}-{:02}", unit.season_id, (unit.season_id + 1) % 100);
    let grade = i32::try_from(record.grade_id).ok();
    let decision = classify_cohort(2027, None, Some(&season_label), grade);
    let date = record
        .result_date
        .get(..10)
        .map_or_else(|| record.result_date.clone(), str::to_owned);
    let mut fields = std::collections::BTreeMap::new();
    fields.insert("athlete_id".to_owned(), record.athlete_id.to_string());
    fields.insert("athlete_name".to_owned(), record.athlete_name.clone());
    fields.insert("school".to_owned(), record.team_name.clone());
    fields.insert("team_name".to_owned(), record.team_name.clone());
    fields.insert("state".to_owned(), record.state.clone());
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
        CohortDecision::Exception(reason) | CohortDecision::Exclude(reason) => Some(CohortException {
            athlete_id: record.athlete_id,
            reason,
            source_url: profile_url,
        }),
        CohortDecision::Include(_) => None,
    };
    Ok((athlete, exception))
}

fn unresolved_from(athlete: &SourceAthlete) -> UnresolvedRecord {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn disabled_manifest_fails_before_network() {
        let error = collect_authorized(
            Path::new("alpha.example.toml"),
            Path::new("target/alpha-test-output"),
            Some(1),
            false,
        )
        .await
        .expect_err("disabled authorization must fail");
        assert!(error.to_string().contains("authorization"));
    }
}
