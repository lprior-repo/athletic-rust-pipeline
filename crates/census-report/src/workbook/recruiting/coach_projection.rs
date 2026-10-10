use crate::export::provenance::{current_coach_contacts, CurrentCoachContacts, SelectedContact};
use census_domain::model::{CanonicalCoach, CoachTenureEvidence, SchoolYear};

pub(in crate::workbook) struct Projection<'a> {
    pub professional: Option<SelectedContact<'a>>,
    pub personal: Option<SelectedContact<'a>>,
    pub name: Option<&'a CoachTenureEvidence>,
    pub state: &'static str,
}

fn of(coach: &CanonicalCoach, year: SchoolYear) -> Projection<'_> {
    match current_coach_contacts(coach, year) {
        Ok(Some(selected)) => current(selected),
        Ok(None) => empty("no_current_claim"),
        Err(crate::export::provenance::ContactSelectionError::MailboxConflict) => {
            empty("mailbox_conflict")
        }
        Err(crate::export::provenance::ContactSelectionError::MissingCaptureUrl) => {
            empty("missing_capture_url")
        }
        Err(crate::export::provenance::ContactSelectionError::Tenure(_)) => {
            empty("invalid_or_conflicting_tenure")
        }
    }
}

pub(in crate::workbook) fn admitted<'a>(
    coach: &'a CanonicalCoach,
    year: SchoolYear,
    school: Option<&super::contact::SchoolContacts>,
) -> Projection<'a> {
    let selected = of(coach, year);
    if selected.state != "current_claim"
        || school.is_some_and(|school| school.coach_admitted(coach))
    {
        selected
    } else {
        empty("school_program_contact_withheld")
    }
}

fn current(selected: CurrentCoachContacts<'_>) -> Projection<'_> {
    Projection {
        professional: selected.professional(),
        personal: selected.personal(),
        name: selected.name_only(),
        state: "current_claim",
    }
}

fn empty(state: &'static str) -> Projection<'static> {
    Projection {
        professional: None,
        personal: None,
        name: None,
        state,
    }
}

pub(in crate::workbook) fn capture(fact: Option<&CoachTenureEvidence>) -> [&str; 3] {
    match fact {
        Some(fact) => [
            fact.source.url.as_deref().map_or("", |url| url),
            fact.source_sha256.as_str(),
            fact.retrieved_at.as_str(),
        ],
        None => ["", "", ""],
    }
}

pub(in crate::workbook) fn tenure_label(
    coach: &CanonicalCoach,
    school_year: SchoolYear,
) -> &'static str {
    use census_domain::model::{CoachTenure, TenureAssessmentError};
    match coach.tenure_state(school_year) {
        Ok(CoachTenure::Current { .. }) => "current_declared",
        Ok(CoachTenure::Former { .. }) => "former_declared",
        Ok(CoachTenure::Unknown) if coach.tenure_evidence.is_empty() => "unknown",
        Ok(CoachTenure::Unknown) => "historical_evidence",
        Err(TenureAssessmentError::Conflict) => "tenure_conflict",
        Err(TenureAssessmentError::InvalidEvidence { .. }) => "invalid_tenure_evidence",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use census_domain::model::{
        CoachRole, CoachTenure, CoachTenureEvidence, Gender, SchoolId, SchoolYear, SourceRef, Sport,
    };

    fn fixture_coach() -> CanonicalCoach {
        let school = SchoolId::mint("sch", &["test coach fixture"]);
        CanonicalCoach::new(
            &school,
            "Test Coach",
            Some(Sport::OutdoorTrack),
            Gender::Mixed,
            CoachRole::HeadCoach,
        )
    }

    fn tenure_evidence(tenure: CoachTenure, retrieved_at: &str) -> CoachTenureEvidence {
        CoachTenureEvidence {
            tenure,
            source: SourceRef::new("test_source", None),
            source_sha256: "00".repeat(32),
            retrieved_at: retrieved_at.into(),
            statement: "test statement".into(),
            claim: None,
        }
    }

    #[test]
    fn current_tenure_labels_current_declared() {
        let mut coach = fixture_coach();
        coach.tenure_evidence.push(tenure_evidence(
            CoachTenure::Current {
                school_year: SchoolYear::DEFAULT,
            },
            "2026-09-01T00:00:00Z",
        ));
        assert_eq!(
            tenure_label(&coach, SchoolYear::DEFAULT),
            "current_declared"
        );
    }

    #[test]
    fn former_tenure_labels_former_declared() {
        let mut coach = fixture_coach();
        coach.tenure_evidence.push(tenure_evidence(
            CoachTenure::Former {
                last_school_year: Some(SchoolYear::DEFAULT),
            },
            "2026-09-01T00:00:00Z",
        ));
        assert_eq!(tenure_label(&coach, SchoolYear::DEFAULT), "former_declared");
    }

    #[test]
    fn unknown_tenure_no_evidence_labels_unknown() {
        let coach = fixture_coach();
        assert_eq!(tenure_label(&coach, SchoolYear::DEFAULT), "unknown");
    }

    #[test]
    fn unknown_tenure_with_evidence_labels_historical_evidence() {
        let mut coach = fixture_coach();
        coach.tenure_evidence.push(tenure_evidence(
            CoachTenure::Unknown,
            "2020-01-01T00:00:00Z",
        ));
        assert_eq!(
            tenure_label(&coach, SchoolYear::DEFAULT),
            "historical_evidence"
        );
    }

    #[test]
    fn conflicting_tenure_labels_tenure_conflict() {
        let mut coach = fixture_coach();
        coach.tenure_evidence.push(tenure_evidence(
            CoachTenure::Current {
                school_year: SchoolYear::DEFAULT,
            },
            "2026-09-01T00:00:00Z",
        ));
        coach.tenure_evidence.push(tenure_evidence(
            CoachTenure::Former {
                last_school_year: Some(SchoolYear::DEFAULT),
            },
            "2026-08-01T00:00:00Z",
        ));
        assert_eq!(tenure_label(&coach, SchoolYear::DEFAULT), "tenure_conflict");
    }

    #[test]
    fn invalid_tenure_evidence_labels_invalid() {
        let mut coach = fixture_coach();
        let mut evidence = tenure_evidence(
            CoachTenure::Current {
                school_year: SchoolYear::DEFAULT,
            },
            "not-valid-rfc3339",
        );
        evidence.source_sha256 = "not-hex".into();
        coach.tenure_evidence.push(evidence);
        assert_eq!(
            tenure_label(&coach, SchoolYear::DEFAULT),
            "invalid_tenure_evidence"
        );
    }
}
