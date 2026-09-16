use crate::alpha_api_client::AlphaApiClient;
use crate::alpha_catalog::parse_nav_targets;
use crate::alpha_checkpoint::{append, load_latest, AlphaCheckpoint, AlphaUnitKey};
use crate::alpha_config::AlphaConfig;
use crate::alpha_merge::merge_athlete;
use crate::alpha_model::{AlphaRequest, RunMatrix, RunUnit, SourceAthlete};
use crate::alpha_output::{
    read_jsonl, write_outputs, CohortException, CoverageReport, UnresolvedRecord,
};
use crate::alpha_pipeline_records::{source_athlete, unresolved_from};
use anyhow::{bail, Context, Result};
use std::collections::{BTreeMap, BTreeSet};
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
    let authorized_sport = config
        .authorization
        .allowed_sports
        .iter()
        .find(|sport| {
            let lower = sport.to_ascii_lowercase();
            lower.contains("track") && lower.contains("field")
        })
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Track and Field is not authorized"))?;
    let client = AlphaApiClient::new(config.to_client_config())?;
    let nav = client
        .nav_info(
            config
                .authorization
                .allowed_seasons
                .first()
                .copied()
                .context("authorization.allowed_seasons must not be empty")?,
            false,
        )
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
        expected_units: matrix.units().len(),
        bounded: max_units.is_some(),
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
            coverage.complete_units = coverage
                .complete_units
                .checked_add(1)
                .context("complete unit count overflow")?;
            continue;
        }
        let mut continuation = None;
        let mut seen_continuations = BTreeSet::new();
        let mut unit_keyed = BTreeMap::new();
        let mut unit_exceptions = Vec::new();
        let mut unit_unresolved = Vec::new();
        let mut unit_has_exception = false;
        loop {
            let key = unit_key(&unit, &continuation).map_err(|error| anyhow::anyhow!(error))?;
            if !seen_continuations.insert(key.continuation.clone()) {
                append_checkpoint(output_dir, key, 0, false, "loop")?;
                unresolved.push(UnresolvedRecord {
                    record_key: "alpha-unit".to_owned(),
                    reason: "authorized alpha continuation cycle detected".to_owned(),
                    source_url: String::new(),
                });
                persist(
                    output_dir,
                    &keyed,
                    &cohort_exceptions,
                    &unresolved,
                    &coverage,
                )?;
                bail!("authorized alpha continuation cycle detected");
            }
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
                    unresolved.push(UnresolvedRecord {
                        record_key: "alpha-unit".to_owned(),
                        reason: "authorized alpha request failed; retryable".to_owned(),
                        source_url: String::new(),
                    });
                    persist(
                        output_dir,
                        &keyed,
                        &cohort_exceptions,
                        &unresolved,
                        &coverage,
                    )?;
                    return Err(anyhow::anyhow!(error));
                }
            };
            let response_count = page.records.len();
            for record in page.records {
                match source_athlete(&record, &unit, &authorized_sport) {
                    Ok((_athlete, Some(exception))) => {
                        unit_has_exception = true;
                        unit_exceptions.push(exception);
                    }
                    Ok((athlete, None)) if athlete.athlete_id == 0 => {
                        unit_has_exception = true;
                        unit_unresolved.push(unresolved_from(&athlete));
                    }
                    Ok((athlete, None)) if !athlete.exception_notes.is_empty() => {
                        unit_has_exception = true;
                        unit_unresolved.push(unresolved_from(&athlete));
                    }
                    Ok((athlete, None)) => {
                        merge_athlete(&mut unit_keyed, athlete);
                    }
                    Err(reason) => {
                        unit_has_exception = true;
                        unit_unresolved.push(UnresolvedRecord {
                            record_key: "ranking-record".to_owned(),
                            reason,
                            source_url: String::new(),
                        });
                    }
                }
            }
            if page.complete {
                for athlete in unit_keyed.into_values() {
                    merge_athlete(&mut keyed, athlete);
                }
                cohort_exceptions.extend(unit_exceptions);
                unresolved.extend(unit_unresolved);
                if unit_has_exception {
                    coverage.exception_units = coverage
                        .exception_units
                        .checked_add(1)
                        .context("exception unit count overflow")?;
                }
                if response_count == 0 {
                    coverage.empty_units = coverage
                        .empty_units
                        .checked_add(1)
                        .context("empty unit count overflow")?;
                }
                persist(
                    output_dir,
                    &keyed,
                    &cohort_exceptions,
                    &unresolved,
                    &coverage,
                )?;
                append_checkpoint(output_dir, key, response_count, true, "complete")?;
                coverage.complete_units = coverage
                    .complete_units
                    .checked_add(1)
                    .context("complete unit count overflow")?;
                break;
            }
            let Some(next) = page.continuation else {
                append_checkpoint(output_dir, key, response_count, false, "incomplete")?;
                unresolved.push(UnresolvedRecord {
                    record_key: "alpha-unit".to_owned(),
                    reason: "authorized alpha unit is incomplete without continuation".to_owned(),
                    source_url: String::new(),
                });
                persist(
                    output_dir,
                    &keyed,
                    &cohort_exceptions,
                    &unresolved,
                    &coverage,
                )?;
                bail!("authorized alpha unit is incomplete without continuation");
            };
            let next_key =
                unit_key(&unit, &Some(next.clone())).map_err(|error| anyhow::anyhow!(error))?;
            append_checkpoint(output_dir, next_key, response_count, false, "incomplete")?;
            continuation = Some(next);
        }
    }
    let athletes: Vec<SourceAthlete> = keyed.into_values().collect();
    write_outputs(
        output_dir,
        &athletes,
        &cohort_exceptions,
        &unresolved,
        &coverage,
    )?;
    Ok(CollectionSummary {
        coverage,
        athlete_count: athletes.len(),
        cohort_exception_count: cohort_exceptions.len(),
        unresolved_count: unresolved.len(),
    })
}
fn persist(
    output_dir: &Path,
    keyed: &BTreeMap<u64, SourceAthlete>,
    cohort_exceptions: &[CohortException],
    unresolved: &[UnresolvedRecord],
    coverage: &CoverageReport,
) -> Result<()> {
    let athletes: Vec<SourceAthlete> = keyed.values().cloned().collect();
    write_outputs(
        output_dir,
        &athletes,
        cohort_exceptions,
        unresolved,
        coverage,
    )
}

fn unit_key(
    unit: &RunUnit,
    continuation: &Option<serde_json::Value>,
) -> Result<AlphaUnitKey, String> {
    let continuation = match continuation {
        None => "0".to_owned(),
        Some(value) => {
            serde_json::to_string(value).map_err(|_| "invalid continuation".to_owned())?
        }
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

fn append_checkpoint(
    output_dir: &Path,
    key: AlphaUnitKey,
    response_count: usize,
    complete: bool,
    status: &str,
) -> Result<()> {
    append(
        output_dir,
        &AlphaCheckpoint {
            key,
            response_count,
            complete,
            status: status.to_owned(),
        },
    )
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
