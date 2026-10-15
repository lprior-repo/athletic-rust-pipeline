use super::{PublicIdentityGroup, VerifiedPublicIdentityView};
use census_domain::model::AthleteCandidateId;
use census_domain::public_identity_history::{PublicAthleteId, PublicResolution};
use census_store::{StoreError, StoreResult};
use serde::{ser::SerializeStruct, Serialize, Serializer};

mod resolution;
pub(crate) use resolution::ResolutionAudit;

#[derive(Clone, Copy)]
pub(crate) struct AcceptedIdentityFields<'a> {
    athlete_id: &'a PublicAthleteId,
    canonical_candidate_id: &'a AthleteCandidateId,
    identity_resolution: &'a PublicResolution,
}

impl<'a> AcceptedIdentityFields<'a> {
    pub(crate) fn for_candidate(
        view: &'a VerifiedPublicIdentityView,
        candidate: &str,
    ) -> StoreResult<Self> {
        let group = view.group(candidate).ok_or_else(absent)?;
        let athlete_id = group.public_id().ok_or_else(withheld)?;
        let identity_resolution = view.candidate_resolution(candidate).ok_or_else(absent)?;
        Ok(Self {
            athlete_id,
            canonical_candidate_id: group.canonical_candidate(),
            identity_resolution,
        })
    }

    pub(crate) fn athlete_id(&self) -> &'a PublicAthleteId {
        self.athlete_id
    }

    pub(crate) fn canonical_candidate_id(&self) -> &'a AthleteCandidateId {
        self.canonical_candidate_id
    }

    pub(crate) fn resolution_audit(&self) -> ResolutionAudit<'_> {
        ResolutionAudit(self.identity_resolution)
    }

    pub(crate) fn resolution(&self) -> &'a PublicResolution {
        self.identity_resolution
    }
}

impl Serialize for AcceptedIdentityFields<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut wire = serializer.serialize_struct("accepted_public_identity", 3)?;
        wire.serialize_field("athlete_id", self.athlete_id())?;
        wire.serialize_field("canonical_candidate_id", self.canonical_candidate_id())?;
        wire.serialize_field("identity_resolution", &ResolutionAudit(self.resolution()))?;
        wire.end()
    }
}

#[cfg(test)]
#[derive(Serialize)]
pub(crate) struct IdentityAuditEntry<'a> {
    original_candidate_id: &'a AthleteCandidateId,
    canonical_candidate_id: &'a AthleteCandidateId,
    public_athlete_id: Option<&'a PublicAthleteId>,
    publication_identity_status: &'static str,
    resolution: ResolutionAudit<'a>,
}

#[cfg(test)]
impl VerifiedPublicIdentityView {
    pub(crate) fn identity_audit(&self) -> impl Iterator<Item = IdentityAuditEntry<'_>> {
        self.groups.values().flat_map(|group| {
            group
                .members()
                .map(move |(candidate, resolution)| IdentityAuditEntry {
                    original_candidate_id: candidate,
                    canonical_candidate_id: group.canonical_candidate(),
                    public_athlete_id: group.public_id(),
                    publication_identity_status: status(group, resolution),
                    resolution: ResolutionAudit(resolution),
                })
        })
    }
}

pub(crate) fn status(group: &PublicIdentityGroup, resolution: &PublicResolution) -> &'static str {
    match resolution {
        PublicResolution::Unknown => "unknown",
        PublicResolution::Unresolved => "unresolved",
        PublicResolution::Ambiguous { .. } => "ambiguous",
        _ if group.public_id().is_none() => "candidate_group_unresolved",
        PublicResolution::Canonical(_) => "canonical",
        PublicResolution::Redirect { .. } => "redirect",
        PublicResolution::RevokedRedirect { .. } => "revoked_redirect",
    }
}

fn absent() -> StoreError {
    StoreError::Invariant {
        detail: "selected identity candidate is absent from the retained public identity inventory"
            .into(),
    }
}

fn withheld() -> StoreError {
    StoreError::Invariant {
        detail: "selected identity has no unambiguous retained public athlete ID".into(),
    }
}
