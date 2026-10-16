use crate::model::{CanonicalSchool, SchoolId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureDayAffiliationResolution {
    Resolved {
        school: SchoolId,
        evidence_refs: Vec<String>,
    },
    Conflicted,
    Unknown,
    CoopCampusUnresolved,
}

impl CaptureDayAffiliationResolution {
    pub fn school(&self) -> Option<&SchoolId> {
        match self {
            Self::Resolved { school, .. } => Some(school),
            Self::Conflicted | Self::Unknown | Self::CoopCampusUnresolved => None,
        }
    }

    pub fn status(&self) -> &'static str {
        match self {
            Self::Resolved { .. } => "resolved_at_selected_cutoff",
            Self::Conflicted => "conflicting_affiliation_evidence",
            Self::Unknown => "current_affiliation_unknown",
            Self::CoopCampusUnresolved => "coop_campus_unresolved",
        }
    }

    pub fn evidence_refs(&self) -> &[String] {
        match self {
            Self::Resolved { evidence_refs, .. } => evidence_refs,
            Self::Conflicted | Self::Unknown | Self::CoopCampusUnresolved => &[],
        }
    }

    pub fn observe_school(&mut self, school: &CanonicalSchool, mut evidence_refs: Vec<String>) {
        evidence_refs.sort();
        evidence_refs.dedup();
        let incoming = if school.co_op {
            Self::CoopCampusUnresolved
        } else {
            Self::Resolved {
                school: school.id.clone(),
                evidence_refs,
            }
        };
        match (&mut *self, incoming) {
            (Self::Unknown, incoming) => *self = incoming,
            (
                Self::Resolved {
                    school,
                    evidence_refs,
                },
                Self::Resolved {
                    school: other,
                    evidence_refs: mut refs,
                },
            ) if *school == other => {
                evidence_refs.append(&mut refs);
                evidence_refs.sort();
                evidence_refs.dedup();
            }
            (Self::CoopCampusUnresolved, Self::CoopCampusUnresolved) => {}
            _ => *self = Self::Conflicted,
        }
    }

    pub fn withhold(&mut self) {
        match self {
            Self::Conflicted => {}
            Self::Resolved { .. } | Self::Unknown | Self::CoopCampusUnresolved => {
                *self = Self::Unknown;
            }
        }
    }
}

#[cfg(test)]
mod tests;
