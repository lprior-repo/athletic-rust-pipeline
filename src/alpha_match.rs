use crate::alpha_model::SourceAthlete;
use crate::alpha_url::{validate_profile_url, validate_result_url, validate_source_url};
use crate::config::MatchingConfig;
use crate::marks;
use crate::model::{Candidate, Mark, Prospect};
use crate::scoring;
use anyhow::{Context, Result};
use std::collections::BTreeMap;

pub struct SourceIndex<'a> {
    by_name: BTreeMap<String, Vec<&'a SourceAthlete>>,
}

pub fn build_source_index<'a>(source: &'a [SourceAthlete]) -> SourceIndex<'a> {
    let mut by_name: BTreeMap<String, Vec<&'a SourceAthlete>> = BTreeMap::new();
    for athlete in source {
        let key = name_key(&athlete.athlete_name, &athlete.first_name, &athlete.last_name);
        if !key.is_empty() {
            by_name.entry(key).or_default().push(athlete);
        }
    }
    SourceIndex { by_name }
}

pub fn indexed_candidates_for_prospect(
    prospect: &Prospect,
    index: &SourceIndex<'_>,
    config: &MatchingConfig,
) -> Vec<Candidate> {
    let key = name_key(&prospect.full_name(), "", "");
    index
        .by_name
        .get(&key)
        .into_iter()
        .flatten()
        .map(|athlete| candidate_from_source(athlete, prospect, config))
        .collect()
}

#[allow(dead_code)]
pub fn candidates_for_prospect(
    prospect: &Prospect,
    source: &[SourceAthlete],
    config: &MatchingConfig,
) -> Vec<Candidate> {
    let index = build_source_index(source);
    indexed_candidates_for_prospect(prospect, &index, config)
}

pub fn candidate_from_source(
    athlete: &SourceAthlete,
    _prospect: &Prospect,
    _config: &MatchingConfig,
) -> Candidate {
    let profile_url = canonical_profile(athlete);
    let marks = athlete
        .results
        .iter()
        .filter_map(|source| {
            let mut mark = Mark::default();
            mark.event = source.event.clone();
            mark.mark = source.mark.clone();
            mark.season = source.season.clone();
            mark.date = source.date.clone();
            mark.meet_name = source.meet_name.clone();
            mark.wind = source.wind.clone();
            mark.source_url = source.source_url.clone();
            let normalized = marks::normalize_mark(mark);
            normalized.valid.then_some(normalized)
        })
        .collect();
    let mut evidence_urls = athlete
        .profile_urls
        .iter()
        .filter_map(|url| validate_profile_url(url))
        .chain(athlete.source_urls.iter().filter_map(|url| validate_source_url(url)))
        .chain(
            athlete
                .results
                .iter()
                .filter_map(|result| result.result_url.as_deref())
                .filter_map(validate_result_url),
        )
        .collect::<Vec<_>>();
    evidence_urls.sort();
    evidence_urls.dedup();
    Candidate {
        profile_url: profile_url.clone(),
        athlete_name: if athlete.athlete_name.is_empty() {
            format!("{} {}", athlete.first_name, athlete.last_name).trim().to_owned()
        } else {
            athlete.athlete_name.clone()
        },
        school: athlete.school.clone(),
        location: format_location(&athlete.city, &athlete.state),
        athlete_id: (athlete.athlete_id > 0).then_some(athlete.athlete_id),
        graduation_year: athlete.graduation_year,
        sports: if athlete.sport.is_empty() { Vec::new() } else { vec![athlete.sport.clone()] },
        marks,
        page_retrieved: false,
        evidence_text: athlete.cohort_evidence.clone(),
        evidence_urls,
        ..Default::default()
    }
}
fn canonical_profile(athlete: &SourceAthlete) -> String {
    let candidates = athlete
        .profile_urls
        .iter()
        .map(String::as_str)
        .chain(std::iter::once(athlete.profile_url.as_str()));
    for candidate in candidates {
        if let Some(profile) = validate_profile_url(candidate) {
            return profile;
        }
    }
    String::new()
}

pub fn score_indexed_candidates(
    prospect: &Prospect,
    index: &SourceIndex<'_>,
    config: &MatchingConfig,
) -> Vec<Candidate> {
    let mut candidates = indexed_candidates_for_prospect(prospect, index, config);
    for candidate in &mut candidates {
        scoring::score_candidate(prospect, candidate, config);
    }
    candidates
}
pub fn match_workbook(
    input: &std::path::Path,
    alpha_source: &std::path::Path,
    config_path: &std::path::Path,
    output_dir: &std::path::Path,
    max: Option<usize>,
) -> Result<()> {
    let config = crate::config::Config::load(config_path)?;
    let source_path = if alpha_source.is_dir() {
        alpha_source.join("athletes.jsonl")
    } else {
        alpha_source.to_owned()
    };
    let source = crate::alpha_output::read_jsonl::<SourceAthlete>(&source_path)
        .with_context(|| format!("loading alpha source {}", source_path.display()))?;
    let index = build_source_index(&source);
    let scan = crate::xlsx::scan(
        input,
        &config.workbook.sports,
        config.workbook.expected_graduation_year,
    )?;
    let limit = match max {
        Some(value) => value,
        None => usize::MAX,
    };
    let records = scan
        .prospects
        .into_iter()
        .take(limit)
        .map(|prospect| {
            let candidates = score_indexed_candidates(&prospect, &index, &config.matching);
            crate::scoring::finalize_match(
                prospect,
                candidates,
                crate::model::ModelDecision::default(),
                &config.matching,
            )
        })
        .collect::<Vec<_>>();
    std::fs::create_dir_all(output_dir)
        .with_context(|| format!("creating {}", output_dir.display()))?;
    crate::output::write_all(output_dir, &records)
}

fn format_location(city: &str, state: &str) -> String {
    match (city.trim().is_empty(), state.trim().is_empty()) {
        (true, true) => String::new(),
        (false, true) => city.trim().to_owned(),
        (true, false) => state.trim().to_owned(),
        (false, false) => format!("{}, {}", city.trim(), state.trim()),
    }
}

fn name_key(full: &str, first: &str, last: &str) -> String {
    let value = if full.trim().is_empty() {
        format!("{} {}", first, last)
    } else {
        full.to_owned()
    };
    value
        .split_whitespace()
        .map(|word| word.to_lowercase())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Prospect;

    fn prospect() -> Prospect {
        Prospect {
            first_name: "Test".to_owned(),
            last_name: "Runner".to_owned(),
            school: "Lincoln".to_owned(),
            city: "Phoenix".to_owned(),
            state: "AZ".to_owned(),
            sport: "Track and Field".to_owned(),
            expected_graduation_year: Some(2027),
            ..Default::default()
        }
    }

    #[test]
    fn source_record_becomes_candidate_with_safe_evidence() {
        let source = [SourceAthlete {
            athlete_id: 7,
            athlete_name: "Test Runner".to_owned(),
            school: "Lincoln".to_owned(),
            state: "AZ".to_owned(),
            city: "Phoenix".to_owned(),
            sport: "Track and Field".to_owned(),
            profile_url: "https://athletic.net/athlete/7".to_owned(),
            profile_urls: vec!["https://athletic.net/athlete/7".to_owned()],
            graduation_year: Some(2027),
            ..Default::default()
        }];
        let config = MatchingConfig {
            match_threshold: 0.8,
            close_threshold: 0.6,
            review_threshold: 0.4,
            require_corroboration: true,
        };
        let candidates = candidates_for_prospect(&prospect(), &source, &config);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].athlete_id, Some(7));
        assert_eq!(candidates[0].profile_url, "https://athletic.net/athlete/7");
        assert!(!candidates[0].page_retrieved);
    }
}
