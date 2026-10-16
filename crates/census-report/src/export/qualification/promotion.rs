use super::participation::{ParticipationReplay, PinnedParticipation};
use super::{ParticipationAssessment, ParticipationOutcome, VerifiedParticipationView};
use census_domain::participation_candidates::CandidateOutcome;

impl VerifiedParticipationView {
    pub(crate) fn from_pinned(pinned: PinnedParticipation) -> (Self, ParticipationReplay) {
        let (input_generation, profile_id, participation_rule, rows, replay) = pinned.into_parts();
        let rows = rows
            .into_iter()
            .map(|(id, row)| {
                let (outcome, reasons, evidence_refs, diagnostics) = row.into_parts();
                let outcome = match outcome {
                    CandidateOutcome::EligibleCandidate => ParticipationOutcome::Eligible,
                    CandidateOutcome::Excluded => ParticipationOutcome::Excluded,
                    CandidateOutcome::Unresolved => ParticipationOutcome::Unresolved,
                };
                (
                    id,
                    ParticipationAssessment {
                        outcome,
                        reasons,
                        evidence_refs,
                        diagnostics,
                    },
                )
            })
            .collect();
        (
            Self {
                input_generation,
                profile_id,
                participation_rule,
                rows,
            },
            replay,
        )
    }
}
