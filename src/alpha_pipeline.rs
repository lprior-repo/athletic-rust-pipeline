use crate::alpha_api_client::AlphaApiClient;
use crate::alpha_catalog::parse_nav_targets;
use crate::alpha_checkpoint::{append, load_latest, AlphaCheckpoint, AlphaUnitKey};
use crate::alpha_cohort::{classify_cohort, CohortDecision};
use crate::alpha_config::AlphaConfig;
use crate::alpha_merge::merge_athlete;
use crate::alpha_model::{AlphaRequest, RunMatrix, RunUnit, SourceAthlete};
use crate::alpha_normalize::normalize_record;
use crate::alpha_output::{write_outputs, CohortException, CoverageReport, UnresolvedRecord};
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
    let mut cohort_exceptions = Vec::new();
    let mut unresolved = Vec::new();

    for unit in units {
        let key = unit_key(&unit);
        if latest.get(&key).is_some_and(|state| state.complete) {
            coverage.complete_units += 1;
            continue;
        }
        let page = client
            .rankings(&AlphaRequest {
                state_id: unit.state.state_id,
                season_id: unit.season_id,
                gender: unit.gender.clone(),
                event_short: unit.event.event_short.clone(),
                indoor: false,
                continuation: None,
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
        if !page.complete {
            append_checkpoint(output_dir, key, response_count, false, "incomplete")?;
            bail!("authorized alpha unit is incomplete; resume from checkpoint");
        }
        if response_count == 0 {
            coverage.empty_units += 1;
        }
        for record in page.records {
            match source_athlete(&record, &unit) {
                Ok((_athlete, Some(exception))) => cohort_exceptions.push(exception),
                Ok((athlete, None)) => {
                    let merged = merge_athlete(&mut keyed, athlete);
                    if merged.athlete_id == 0 {
                        unresolved.push(unresolved_from(&merged));
                    }
                }
                Err(reason) => unresolved.push(UnresolvedRecord {
                    record_key: "ranking-record".to_owned(),
                    reason,
                    source_url: String::new(),
                }),
            }
        }
        append_checkpoint(output_dir, key, response_count, true, "complete")?;
        coverage.complete_units += 1;
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

fn unit_key(unit: &RunUnit) -> AlphaUnitKey {
    AlphaUnitKey {
        state_code: unit.state.code.clone(),
        season_id: unit.season_id,
        gender: unit.gender.clone(),
        event_short: unit.event.event_short.clone(),
        continuation: String::new(),
    }
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
