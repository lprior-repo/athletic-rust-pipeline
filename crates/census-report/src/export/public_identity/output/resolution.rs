use census_domain::model::AthleteCandidateId;
use census_domain::public_identity_history::{
    EstablishmentOrdinal, PublicAthleteId, PublicResolution, ResolvedSuccessor,
};
use serde::{Serialize, Serializer};

pub(crate) struct ResolutionAudit<'a>(pub(crate) &'a PublicResolution);

#[derive(Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum Resolution<'a> {
    Unknown,
    Unresolved,
    Canonical {
        successor: Successor<'a>,
    },
    Redirect {
        successor: Successor<'a>,
        changed_at: u64,
    },
    RevokedRedirect {
        successor: Successor<'a>,
        changed_at: u64,
        revoked_merges: &'a [u64],
    },
    Ambiguous {
        successors: Successors<'a>,
        unresolved_bindings: bool,
        changed_at: u64,
        revoked_merges: &'a [u64],
    },
}

#[derive(Serialize)]
struct Successor<'a> {
    public_id: &'a PublicAthleteId,
    representative_candidate: &'a AthleteCandidateId,
    established: &'a EstablishmentOrdinal,
}

impl<'a> From<&'a ResolvedSuccessor> for Successor<'a> {
    fn from(value: &'a ResolvedSuccessor) -> Self {
        Self {
            public_id: &value.public_id,
            representative_candidate: &value.representative_candidate,
            established: &value.established,
        }
    }
}

struct Successors<'a>(&'a [ResolvedSuccessor]);

impl Serialize for Successors<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.iter().map(Successor::from))
    }
}

impl Serialize for ResolutionAudit<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let wire = match self.0 {
            PublicResolution::Unknown => Resolution::Unknown,
            PublicResolution::Unresolved => Resolution::Unresolved,
            PublicResolution::Canonical(successor) => Resolution::Canonical {
                successor: successor.into(),
            },
            PublicResolution::Redirect {
                successor,
                changed_at,
            } => Resolution::Redirect {
                successor: successor.into(),
                changed_at: *changed_at,
            },
            PublicResolution::RevokedRedirect {
                successor,
                changed_at,
                revoked_merges,
            } => Resolution::RevokedRedirect {
                successor: successor.into(),
                changed_at: *changed_at,
                revoked_merges,
            },
            PublicResolution::Ambiguous {
                successors,
                unresolved_bindings,
                changed_at,
                revoked_merges,
            } => Resolution::Ambiguous {
                successors: Successors(successors),
                unresolved_bindings: *unresolved_bindings,
                changed_at: *changed_at,
                revoked_merges,
            },
        };
        wire.serialize(serializer)
    }
}

#[cfg(test)]
mod tests;
