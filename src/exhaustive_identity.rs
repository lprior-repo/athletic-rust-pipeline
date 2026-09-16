use crate::config::MatchingConfig;
use crate::model::{Candidate, MatchRecord, ModelDecision, Prospect};
use crate::scoring;
use url::Url;

/// Extracts the numeric athlete ID from an Athletic.net URL.
/// Returns None if the URL is invalid or does not contain the expected pattern.
fn extract_athlete_id(value: &str) -> Option<u64> {
    let normalized = crate::discovery::allowed_profile_url(value)?;
    let parsed = Url::parse(&normalized).ok()?;
    let mut segments = parsed.path_segments()?;
    if !segments.next()?.eq_ignore_ascii_case("athlete") {
        return None;
    }
    segments.next()?.parse::<u64>().ok().filter(|id| *id > 0)
}

/// Determines if two candidates represent the same athlete identity.
/// They are the same if they share a valid numeric athlete ID.
pub(super) fn is_same_identity(a: &Candidate, b: &Candidate) -> bool {
    match (
        extract_athlete_id(&a.profile_url),
        extract_athlete_id(&b.profile_url),
    ) {
        (Some(id_a), Some(id_b)) => id_a == id_b,
        _ => false,
    }
}

/// Finds the unique candidate if one exists that is significantly better than any
/// competing candidate with a different athlete identity.
///
/// Returns the index of the unique candidate, or None if ambiguous.
pub(super) fn unique_candidate(
    candidates: &[Candidate],
    config: &MatchingConfig,
    margin: f64,
) -> Option<usize> {
    unique_above(candidates, config.match_threshold, margin)
}

fn unique_above(candidates: &[Candidate], threshold: f64, margin: f64) -> Option<usize> {
    if !threshold.is_finite() || !margin.is_finite() || margin < 0.0 {
        return None;
    }
    if candidates.is_empty()
        || candidates.iter().any(|candidate| {
            !candidate.deterministic_score.is_finite()
                || extract_athlete_id(&candidate.profile_url).is_none()
        })
    {
        return None;
    }
    let (index, best) = candidates
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.deterministic_score.total_cmp(&b.deterministic_score))?;
    if !best.corroborated || best.deterministic_score < threshold {
        return None;
    }
    let competitor = candidates.iter().enumerate().any(|(other_index, other)| {
        other_index != index
            && !is_same_identity(best, other)
            && best.deterministic_score - other.deterministic_score <= margin
    });
    if competitor {
        None
    } else {
        Some(index)
    }
}

/// A ranked suggestion is not a calibrated identity probability.
pub(super) fn deterministic_decision(
    candidates: &[Candidate],
    config: &MatchingConfig,
    margin: f64,
) -> ModelDecision {
    let unique = unique_above(candidates, config.close_threshold, margin);
    let best = candidates
        .iter()
        .enumerate()
        .filter(|(_, candidate)| candidate.deterministic_score.is_finite())
        .max_by(|(_, left), (_, right)| {
            left.deterministic_score
                .total_cmp(&right.deterministic_score)
        });
    let decision = match unique.and_then(|index| candidates.get(index)) {
        Some(candidate) if candidate.deterministic_score >= config.match_threshold => "MATCH",
        Some(_) => "CLOSE_MATCH",
        None if candidates.is_empty() => "NO_MATCH",
        None => "REVIEW",
    };
    ModelDecision {
        decision: decision.to_owned(),
        candidate_index: unique.or_else(|| best.map(|(index, _)| index)),
        confidence: best.map_or(0.0, |(_, candidate)| candidate.deterministic_score),
        model_status: "not_run_deterministic".to_owned(),
        reason: "Evidence-only ranking; score is not a probability. Ties, weak evidence and conflicts require review.".to_owned(),
        ..Default::default()
    }
}

/// Finalizes the match record, applying identity checks to demote matches
/// that are ambiguous or involve cross-athlete contamination.
pub(super) fn finalize(
    prospect: Prospect,
    candidates: Vec<Candidate>,
    decision: ModelDecision,
    config: &MatchingConfig,
    margin: f64,
) -> MatchRecord {
    let deterministic_review =
        decision.model_status == "not_run_deterministic" && decision.decision == "REVIEW";
    let mut record = scoring::finalize_match(prospect, candidates, decision, config);
    if deterministic_review {
        // No low-quality heuristic parse may turn a discovered candidate into absence.
        record.status = "REVIEW".to_owned();
        clear_attribution(&mut record);
        return record;
    }
    if matches!(record.status.as_str(), "MATCH" | "CLOSE_MATCH") {
        let threshold = if record.status == "MATCH" {
            config.match_threshold
        } else {
            config.close_threshold
        };
        let best = unique_above(&record.candidates, threshold, margin)
            .and_then(|index| record.candidates.get(index));
        let selected = record
            .model_decision
            .candidate_index
            .and_then(|index| record.candidates.get(index));
        let agreed = matches!((selected, best), (Some(a), Some(b)) if is_same_identity(a, b));
        if !agreed {
            record.status = "REVIEW".to_owned();
            record
                .notes
                .push_str("; Model and unique athlete identity do not agree");
        } else {
            aggregate_sports(&mut record);
        }
    }
    if !matches!(record.status.as_str(), "MATCH" | "CLOSE_MATCH") {
        clear_attribution(&mut record);
    }
    record
}

fn aggregate_sports(record: &mut MatchRecord) {
    let Some(selected) = record
        .selected_candidate_index
        .and_then(|index| record.candidates.get(index))
    else {
        return;
    };
    let sports = record
        .candidates
        .iter()
        .filter(|c| is_same_identity(selected, c))
        .flat_map(|c| &c.sports);
    let (track, xc) = sports.fold((false, false), |(track, xc), sport| {
        let normalized = scoring::normalize(sport);
        (
            track || normalized.contains("track"),
            xc || normalized.contains("cross country"),
        )
    });
    record.track_confirmed = track;
    record.xc_confirmed = xc;
}

fn clear_attribution(record: &mut MatchRecord) {
    record.selected_candidate_index = None;
    record.selected_profile_url.clear();
    record.selected_name.clear();
    record.selected_school.clear();
    record.selected_location.clear();
    record.track_confirmed = false;
    record.xc_confirmed = false;
    record.model_decision.track_confirmed = false;
    record.model_decision.xc_confirmed = false;
    record.best_marks.clear();
}

/// Creates an error record for cases where no match is possible.
pub(super) fn error_record(
    prospect: &Prospect,
    candidates: Vec<Candidate>,
    status: &str,
    reason: String,
) -> MatchRecord {
    MatchRecord {
        source_key: prospect.source_key.clone(),
        deterministic_decision: None,
        prospect: prospect.clone(),
        status: status.to_owned(),
        hint_count: candidates
            .iter()
            .filter(|c| !c.profile_url.trim().is_empty())
            .count(),
        ai_logic: reason.clone(),
        score: 0.0,
        selected_candidate_index: None,
        selected_profile_url: String::new(),
        selected_name: String::new(),
        selected_school: String::new(),
        selected_location: String::new(),
        track_confirmed: false,
        xc_confirmed: false,
        best_marks: Default::default(),
        candidates,
        model_decision: ModelDecision {
            decision: "ERROR".to_owned(),
            candidate_index: None,
            confidence: 0.0,
            track_confirmed: false,
            xc_confirmed: false,
            reason,
            model_status: status.to_owned(),
        },
        notes: String::new(),
        processed_at_unix: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Candidate, Prospect};

    fn test_config() -> MatchingConfig {
        MatchingConfig {
            match_threshold: 0.90,
            close_threshold: 0.80,
            review_threshold: 0.70,
            require_corroboration: true,
        }
    }

    fn make_candidate(id: Option<u64>, score: f64, name: &str, sports: &[&str]) -> Candidate {
        Candidate {
            athlete_name: name.to_owned(),
            profile_url: match id {
                Some(uid) => format!("https://www.athletic.net/athlete/{}/track", uid),
                None => "https://www.athletic.net/athlete/invalid".to_owned(),
            },
            deterministic_score: score,
            corroborated: true,
            sports: sports.iter().map(|sport| (*sport).to_owned()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn test_ambiguous_tied_different_ids_returns_none() {
        let c1 = make_candidate(Some(1), 0.95, "John Doe", &["Track"]);
        let c2 = make_candidate(Some(2), 0.94, "John Doe", &["Track"]);
        let candidates = vec![c1, c2];
        let config = test_config();

        // Margin 0.02 means gap of 0.01 is not enough to be unique
        let result = unique_candidate(&candidates, &config, 0.02);
        assert_eq!(result, None);
    }

    #[test]
    fn test_same_id_different_sports_returns_unique() {
        let c1 = make_candidate(Some(1), 0.95, "Jane Doe", &["Track"]);
        let c2 = make_candidate(Some(1), 0.90, "Jane Doe", &["Cross Country"]);
        let candidates = vec![c1, c2];
        let config = test_config();

        // c1 is best. c2 is same ID. No different ID competitor.
        let result = unique_candidate(&candidates, &config, 0.02);
        assert_eq!(result, Some(0));
    }

    #[test]
    fn test_no_corroboration_returns_none() {
        let mut c1 = make_candidate(Some(1), 0.95, "Bob Smith", &["Track"]);
        c1.corroborated = false;
        let candidates = vec![c1];
        let config = test_config();

        let result = unique_candidate(&candidates, &config, 0.02);
        assert_eq!(result, None);
    }

    #[test]
    fn test_error_record_structure() {
        let prospect = Prospect {
            source_key: "test".to_owned(),
            first_name: "Test".to_owned(),
            last_name: "User".to_owned(),
            ..Default::default()
        };
        // Candidate with empty URL (default)
        let candidates = vec![Candidate::default()];
        let record = error_record(&prospect, candidates, "ERROR", "Reason".to_owned());

        assert_eq!(record.status, "ERROR");
        assert_eq!(record.model_decision.model_status, "ERROR");
        assert_eq!(record.model_decision.reason, "Reason");
        // Empty URL candidate should not contribute to hint_count
        assert_eq!(record.hint_count, 0);
        assert_eq!(record.model_decision.decision, "ERROR");
        assert_eq!(record.selected_candidate_index, None);
    }

    #[test]
    fn same_numeric_id_across_track_and_cross_country_is_one_identity() {
        let track = make_candidate(Some(7), 0.95, "Ada Runner", &["Track & Field"]);
        let xc = Candidate {
            profile_url: "https://www.athletic.net/athlete/0007/cross-country?tab=results"
                .to_owned(),
            ..make_candidate(Some(7), 0.94, "Ada Runner", &["Cross Country"])
        };

        assert!(is_same_identity(&track, &xc));
    }

    #[test]
    fn zero_margin_still_rejects_different_id_tie() {
        let candidates = vec![
            make_candidate(Some(1), 0.95, "Alex Runner", &["Track"]),
            make_candidate(Some(2), 0.95, "Alex Runner", &["Cross Country"]),
        ];

        assert_eq!(unique_candidate(&candidates, &test_config(), 0.0), None);
    }

    #[test]
    fn invalid_url_and_nonfinite_score_are_not_unique() {
        let invalid_url = make_candidate(None, 0.99, "Alex Runner", &["Track"]);
        assert_eq!(unique_candidate(&[invalid_url], &test_config(), 0.0), None);

        let nonfinite = make_candidate(Some(1), f64::NAN, "Alex Runner", &["Track"]);
        assert_eq!(unique_candidate(&[nonfinite], &test_config(), 0.0), None);
    }

    #[test]
    fn model_index_must_agree_with_unique_identity() {
        let config = test_config();
        let candidates = vec![
            make_candidate(Some(1), 0.95, "Alex Runner", &["Track"]),
            make_candidate(Some(2), 0.94, "Alex Runner", &["Cross Country"]),
        ];
        let decision = ModelDecision {
            decision: "MATCH".to_owned(),
            candidate_index: Some(1),
            track_confirmed: true,
            ..Default::default()
        };

        let record = finalize(Prospect::default(), candidates, decision, &config, 0.0);

        assert_eq!(record.status, "REVIEW");
        assert!(record.selected_profile_url.is_empty());
        assert!(!record.track_confirmed && !record.xc_confirmed);
        assert!(!record.model_decision.track_confirmed);
        assert!(!record.model_decision.xc_confirmed);
        assert!(record.best_marks.is_empty());
    }

    #[test]
    fn same_identity_sports_are_aggregated_from_candidates() {
        let config = test_config();
        let candidates = vec![
            make_candidate(Some(1), 0.95, "Alex Runner", &["Track & Field"]),
            make_candidate(Some(1), 0.94, "Alex Runner", &["Cross Country"]),
        ];
        let decision = ModelDecision {
            decision: "MATCH".to_owned(),
            candidate_index: Some(0),
            ..Default::default()
        };

        let record = finalize(Prospect::default(), candidates, decision, &config, 0.0);

        assert_eq!(record.status, "MATCH");
        assert!(record.track_confirmed);
        assert!(record.xc_confirmed);
    }
}
