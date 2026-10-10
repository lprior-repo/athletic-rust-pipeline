use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, Evidence, EvidenceMethod, RelayResult, RetainedConflict, ReviewCase,
    SourceObservation,
};
use serde::Serialize;

pub(crate) trait CanonicalProjection: Serialize {
    fn canonicalize_sets(&mut self);
}

impl CanonicalProjection for CanonicalMeet {
    fn canonicalize_sets(&mut self) {
        self.sports.sort_unstable();
        self.source_identities.sort_unstable();
        self.source_urls.sort_unstable();
        provenance(&mut self.evidence, &mut self.retained_conflicts);
    }
}

impl CanonicalProjection for CanonicalEvent {
    fn canonicalize_sets(&mut self) {
        self.source_labels.sort_unstable_by(|left, right| {
            (&left.source, &left.label).cmp(&(&right.source, &right.label))
        });
        provenance(&mut self.evidence, &mut self.retained_conflicts);
    }
}

impl CanonicalProjection for CanonicalTeam {
    fn canonicalize_sets(&mut self) {
        self.source_identities.sort_unstable();
        provenance(&mut self.evidence, &mut self.retained_conflicts);
    }
}

impl CanonicalProjection for CanonicalAthlete {
    fn canonicalize_sets(&mut self) {
        self.known_names.sort_unstable();
        self.sports.sort_unstable();
        self.source_links.sort_unstable();
        self.published_graduations.sort_unstable_by(|left, right| {
            (left.grad_year, &left.source).cmp(&(right.grad_year, &right.source))
        });
        provenance(&mut self.evidence, &mut self.retained_conflicts);
    }
}

impl CanonicalProjection for CanonicalPerformance {
    fn canonicalize_sets(&mut self) {
        provenance(&mut self.evidence, &mut self.retained_conflicts);
    }
}

impl CanonicalProjection for RelayResult {
    fn canonicalize_sets(&mut self) {
        self.members.sort_by_key(|member| member.order);
        provenance(&mut self.evidence, &mut []);
    }
}

impl CanonicalProjection for SourceObservation {
    fn canonicalize_sets(&mut self) {}
}

impl CanonicalProjection for CanonicalSchool {
    fn canonicalize_sets(&mut self) {
        self.aliases.sort_unstable();
        self.source_identities.sort_unstable();
        provenance(&mut self.evidence, &mut self.retained_conflicts);
    }
}

impl CanonicalProjection for ReviewCase {
    fn canonicalize_sets(&mut self) {}
}

fn provenance(evidence: &mut [Evidence], conflicts: &mut [RetainedConflict]) {
    evidence.sort_unstable_by(|left, right| {
        (
            &left.source,
            method_key(left.method),
            &left.observed_on,
            &left.note,
        )
            .cmp(&(
                &right.source,
                method_key(right.method),
                &right.observed_on,
                &right.note,
            ))
    });
    conflicts.sort_unstable_by(|left, right| {
        (
            &left.id,
            &left.family,
            &left.subject_id,
            &left.subject,
            &left.detail,
        )
            .cmp(&(
                &right.id,
                &right.family,
                &right.subject_id,
                &right.subject,
                &right.detail,
            ))
    });
}

const fn method_key(method: EvidenceMethod) -> u8 {
    match method {
        EvidenceMethod::Fetched => 0,
        EvidenceMethod::Parsed => 1,
        EvidenceMethod::Derived => 2,
    }
}
