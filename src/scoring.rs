use crate::{
    config::MatchingConfig,
    marks,
    model::{Candidate, MatchRecord, ModelDecision, Prospect},
};
use strsim::jaro_winkler;
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

pub fn score_candidate(prospect: &Prospect, candidate: &mut Candidate, config: &MatchingConfig) {
    let source_name = normalize(&prospect.full_name());
    let candidate_name = normalize(&candidate.athlete_name);
    let source_school = normalize(&prospect.school);
    let candidate_school = normalize(&candidate.school);
    candidate.name_score = similarity(&source_name, &candidate_name);
    candidate.school_score = similarity(&source_school, &candidate_school);
    candidate.location_score = location_score(prospect, &candidate.location);

    let year_score = match (prospect.expected_graduation_year, candidate.graduation_year) {
        (Some(expected), Some(actual)) if expected == actual => 1.0,
        (Some(_), Some(_)) => 0.0,
        _ => 0.0,
    };
    let sport_score = sport_score(&prospect.sport, &candidate.sports);
    candidate.corroborated =
        candidate.school_score >= 0.82 || candidate.location_score >= 0.90 || year_score == 1.0;

    let mut score = candidate.name_score * 0.68
        + candidate.school_score * 0.20
        + candidate.location_score * 0.07
        + year_score * 0.03
        + sport_score * 0.02;

    if prospect.expected_graduation_year.is_some()
        && candidate.graduation_year.is_some()
        && year_score == 0.0
    {
        score -= 0.12;
    }
    if sport_score == 0.0 && !candidate.sports.is_empty() {
        score -= 0.10;
    }
    if config.require_corroboration && !candidate.corroborated {
        score = score.min(config.review_threshold + 0.07);
    }
    candidate.deterministic_score = score.clamp(0.0, 1.0);
}

fn no_candidate_record(prospect: Prospect, model_decision: ModelDecision) -> MatchRecord {
    MatchRecord {
        source_key: prospect.source_key.clone(),
        prospect,
        status: "NO_MATCH".to_owned(),
        hint_count: 0,
        ai_logic: "No AI candidate review: search returned no Athletic.net profile hint."
            .to_owned(),
        model_decision,
        notes: "No candidate profile URL discovered".to_owned(),
        ..Default::default()
    }
}

pub fn finalize_match(
    prospect: Prospect,
    candidates: Vec<Candidate>,
    model_decision: ModelDecision,
    config: &MatchingConfig,
) -> MatchRecord {
    if candidates.is_empty() {
        return no_candidate_record(prospect, model_decision);
    }

    let Some((deterministic_best, _)) =
        candidates
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| {
                left.deterministic_score
                    .total_cmp(&right.deterministic_score)
            })
    else {
        return no_candidate_record(prospect, model_decision);
    };
    let selected_index = match model_decision.candidate_index.filter(|index| {
        let Some(candidate) = candidates.get(*index) else {
            return false;
        };
        let Some(best) = candidates.get(deterministic_best) else {
            return false;
        };
        candidate.deterministic_score + 0.05 >= best.deterministic_score && candidate.corroborated
    }) {
        Some(index) => index,
        None => deterministic_best,
    };
    let Some(selected) = candidates.get(selected_index) else {
        return no_candidate_record(prospect, model_decision);
    };
    let mut status = status_for_score(selected.deterministic_score, config).to_owned();

    if config.require_corroboration && !selected.corroborated && status != "NO_MATCH" {
        status = "REVIEW".to_owned();
    }
    status = demote_from_model(&status, &model_decision.decision).to_owned();
    let track_confirmed = selected
        .sports
        .iter()
        .any(|sport| normalize(sport).contains("track"))
        || (model_decision.candidate_index == Some(selected_index)
            && model_decision.track_confirmed);
    let xc_confirmed = selected
        .sports
        .iter()
        .any(|sport| normalize(sport).contains("cross country"))
        || (model_decision.candidate_index == Some(selected_index) && model_decision.xc_confirmed);
    let best_marks = marks::best_marks(&selected.marks);
    let notes = format!(
        "Deterministic {:.3}; model {} ({}, {:.3}): {}",
        selected.deterministic_score,
        model_decision.decision,
        model_decision.model_status,
        model_decision.confidence,
        model_decision.reason
    );

    let hint_count = candidates
        .iter()
        .filter(|candidate| !candidate.profile_url.trim().is_empty())
        .count();
    let ai_logic = format!(
        "AI decision={} confidence={:.3} status={} reason={}; Rust scores name={:.3} school={:.3} location={:.3} deterministic={:.3} corroborated={} hint_count={}",
        model_decision.decision,
        model_decision.confidence,
        model_decision.model_status,
        model_decision.reason,
        selected.name_score,
        selected.school_score,
        selected.location_score,
        selected.deterministic_score,
        selected.corroborated,
        hint_count
    );
    MatchRecord {
        source_key: prospect.source_key.clone(),
        prospect,
        status,
        hint_count,
        ai_logic,
        score: selected.deterministic_score,
        selected_candidate_index: Some(selected_index),
        selected_profile_url: selected.profile_url.clone(),
        selected_name: selected.athlete_name.clone(),
        selected_school: selected.school.clone(),
        selected_location: selected.location.clone(),
        track_confirmed,
        xc_confirmed,
        best_marks,
        candidates,
        model_decision,
        notes,
        processed_at_unix: 0,
    }
}

fn status_for_score(score: f64, config: &MatchingConfig) -> &'static str {
    if score >= config.match_threshold {
        "MATCH"
    } else if score >= config.close_threshold {
        "CLOSE_MATCH"
    } else if score >= config.review_threshold {
        "REVIEW"
    } else {
        "NO_MATCH"
    }
}

fn demote_from_model<'a>(deterministic: &'a str, model: &str) -> &'a str {
    let deterministic_rank = status_rank(deterministic);
    let model_rank = status_rank(model);
    if model_rank < deterministic_rank {
        match model_rank {
            3 => "MATCH",
            2 => "CLOSE_MATCH",
            1 => "REVIEW",
            0 => "NO_MATCH",
            _ => deterministic,
        }
    } else {
        deterministic
    }
}

fn status_rank(status: &str) -> i32 {
    match status {
        "MATCH" => 3,
        "CLOSE_MATCH" => 2,
        "REVIEW" => 1,
        "NO_MATCH" => 0,
        _ => 4,
    }
}

fn sport_score(requested: &str, sports: &[String]) -> f64 {
    let requested = normalize(requested);
    let wants_track = requested.contains("track");
    let wants_xc = requested.contains("cross country");
    if sports.iter().any(|sport| {
        let sport = normalize(sport);
        (wants_track && sport.contains("track")) || (wants_xc && sport.contains("cross country"))
    }) {
        1.0
    } else {
        0.0
    }
}

fn location_score(prospect: &Prospect, candidate_location: &str) -> f64 {
    let candidate = normalize(candidate_location);
    let city = normalize(&prospect.city);
    let state = normalize(&prospect.state);
    if candidate.is_empty() {
        0.0
    } else if (!city.is_empty() && candidate.contains(&city))
        || (!state.is_empty() && candidate.contains(&state))
    {
        1.0
    } else {
        similarity(&format!("{city} {state}"), &candidate)
    }
}

fn similarity(left: &str, right: &str) -> f64 {
    if left.is_empty() || right.is_empty() {
        0.0
    } else {
        jaro_winkler(left, right)
    }
}

fn normalize(value: &str) -> String {
    value
        .nfkd()
        .filter(|character| !is_combining_mark(*character))
        .map(|character| {
            if character.is_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> MatchingConfig {
        MatchingConfig {
            match_threshold: 0.93,
            close_threshold: 0.86,
            review_threshold: 0.75,
            require_corroboration: true,
        }
    }

    #[test]
    fn exact_name_without_corroboration_is_not_a_match() {
        let prospect = Prospect {
            first_name: "Jordan".to_owned(),
            last_name: "Smith".to_owned(),
            school: "Central High".to_owned(),
            city: "Springfield".to_owned(),
            state: "IL".to_owned(),
            sport: "Men's Track & Field".to_owned(),
            expected_graduation_year: Some(2027),
            ..Default::default()
        };
        let mut candidate = Candidate {
            athlete_name: "Jordan Smith".to_owned(),
            school: "Westview High".to_owned(),
            location: "Portland, OR".to_owned(),
            sports: vec!["Track & Field".to_owned()],
            ..Default::default()
        };
        score_candidate(&prospect, &mut candidate, &config());
        assert!(!candidate.corroborated);
        assert!(candidate.deterministic_score < config().close_threshold);
    }

    #[test]
    fn preserves_a_weak_profile_as_a_visible_hint() {
        let prospect = Prospect {
            first_name: "Jordan".to_owned(),
            last_name: "Smith".to_owned(),
            sport: "Track and Field: Mens".to_owned(),
            ..Default::default()
        };
        let candidate = Candidate {
            profile_url: "https://www.athletic.net/athlete/123/track-and-field".to_owned(),
            athlete_name: "Jordan Smith".to_owned(),
            sports: vec!["Track & Field".to_owned()],
            ..Default::default()
        };
        let record = finalize_match(
            prospect,
            vec![candidate],
            ModelDecision {
                decision: "NO_MATCH".to_owned(),
                reason: "weak corroboration".to_owned(),
                ..Default::default()
            },
            &config(),
        );
        assert_eq!(record.hint_count, 1);
        assert!(record.ai_logic.contains("weak corroboration"));
    }
}
