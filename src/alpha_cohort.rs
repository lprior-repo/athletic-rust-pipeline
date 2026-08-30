/// Cohort classification for class-of-target-year filtering.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CohortDecision {
    Include(String),
    Exclude(String),
    Exception(String),
}

impl CohortDecision {
    pub fn message(&self) -> &str {
        match self {
            Self::Include(m) | Self::Exclude(m) | Self::Exception(m) => m,
        }
    }
}

/// Classify whether a source record belongs to the target cohort.
///
/// Precedence (first match wins):
/// 1. explicit_year == target_year → Include
/// 2. explicit_year != target_year AND grade present → Exception (conflict)
/// 3. Fallback: grade 11 in "2025-26" or grade 12 in "2026-27" → Include
/// 4. Missing/conflicting evidence → Exception
pub fn classify_cohort(
    target_year: i32,
    explicit_year: Option<i32>,
    season_label: Option<&str>,
    grade: Option<i32>,
) -> CohortDecision {
    // Rule 1: explicit graduation year matches target
    if let Some(yr) = explicit_year {
        if yr == target_year {
            return CohortDecision::Include(format!(
                "explicit graduation year {yr} matches target {target_year}"
            ));
        } else {
            // Rule 2: explicit year conflicts, even with grade present
            if grade.is_some() {
                return CohortDecision::Exception(format!(
                    "explicit graduation year {yr} conflicts with target {target_year}"
                ));
            }
            return CohortDecision::Exception(format!(
                "explicit graduation year {yr} does not match target {target_year}"
            ));
        }
    }

    // Rule 3: season/grade fallback
    if let (Some(season), Some(gr)) = (season_label, grade) {
        let normalized = season.trim().to_lowercase();
        if gr == 11 && normalized == "2025-26" {
            return CohortDecision::Include(
                "grade 11 in 2025-26 season matches cohort fallback".to_owned(),
            );
        }
        if gr == 12 && normalized == "2026-27" {
            return CohortDecision::Include(
                "grade 12 in 2026-27 season matches cohort fallback".to_owned(),
            );
        }
    }

    // Rule 4: missing/conflicting evidence
    let mut evidence = String::from("no matching evidence");
    if let Some(yr) = explicit_year {
        evidence = format!("explicit year {yr} present");
    }
    if let Some(gr) = grade {
        if evidence != "no matching evidence" {
            evidence.push_str(", ");
        }
        evidence.push_str(&format!("grade {gr}"));
    }
    if let Some(s) = season_label {
        if !evidence.contains("season") {
            if evidence != "no matching evidence" {
                evidence.push_str(", ");
            }
            evidence.push_str(&format!("season '{}'", s));
        }
    }
    CohortDecision::Exception(evidence)
}
