/// Cohort classification for class-of-target-year filtering.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CohortDecision {
    /// Includes the record in the target cohort.
    Include(String),
    /// Excludes the record (non-target graduation year, no grade conflict).
    Exclude(String),
    /// Requires manual review (explicit year + grade conflict).
    Exception(String),
}

impl CohortDecision {
    pub fn message(&self) -> &str {
        match self {
            Self::Include(m) | Self::Exclude(m) | Self::Exception(m) => m,
        }
    }

    /// Returns true if this decision requires manual review.
    pub fn is_exception(&self) -> bool {
        matches!(self, Self::Exception(_))
    }
}

/// Classify whether a source record belongs to the target cohort.
///
/// Precedence (first match wins):
/// 1. explicit_year == target_year → Include
/// 2. explicit_year != target_year AND grade present → Exception (conflict)
/// 3. explicit_year != target_year AND grade absent → Exclude
/// 4. Fallback: grade 11 in "2025-26" or grade 12 in "2026-27" → Include
/// 5. No explicit year, grade present but no matching season → Exception
/// 6. Missing evidence → Exception
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
            // Rule 2 or 3: explicit year conflicts
            if grade.is_some() {
                return CohortDecision::Exception(format!(
                    "explicit graduation year {yr} conflicts with grade {grade:?} against target {target_year}"
                ));
            } else {
                return CohortDecision::Exclude(format!(
                    "explicit graduation year {yr} does not match target {target_year}"
                ));
            }
        }
    }

    // Rule 4: season/grade fallback for class-of-year estimation
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
        // Grade present but season doesn't match fallback
        return CohortDecision::Exception(format!(
            "grade {gr} with season '{}' does not match cohort fallback",
            season.trim()
        ));
    }

    // Rule 5/6: missing evidence
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
