use super::{
    serialized_digest, CanonicalJsonError, CompetitionCategory, EventId, EventKind,
    EventSpecification, Gender, Id, MeetId, SpecificationError,
};
use serde::{ser::SerializeStruct, Serialize, Serializer};

pub struct EventIdentity<'a> {
    pub meet: &'a MeetId,
    pub kind: EventKind,
    pub gender: Gender,
    pub division: Option<&'a str>,
    pub round: Option<&'a str>,
}

#[derive(Debug, thiserror::Error)]
pub enum EventIdentityError {
    #[error(transparent)]
    Specification(#[from] SpecificationError),
    #[error(transparent)]
    Canonical(#[from] CanonicalJsonError),
}

impl EventIdentity<'_> {
    pub(super) fn mint(
        &self,
        specification: &EventSpecification,
    ) -> Result<EventId, EventIdentityError> {
        specification.validate()?;
        let category = specification.category_in_context(self.gender, self.division)?;
        let specification = SpecificationIdentity {
            specification,
            category,
        };
        let digest = serialized_digest(&specification)?;
        let kind = self.kind.stable_key();
        let gender = match self.gender {
            Gender::Boys => "m",
            Gender::Girls => "f",
            _ => "u",
        };
        Ok(Id::mint(
            "evt",
            &[
                self.meet.as_str(),
                kind.as_ref(),
                gender,
                self.division.map_or("", core::convert::identity),
                self.round.map_or("", core::convert::identity),
                &digest,
            ],
        ))
    }
}

struct SpecificationIdentity<'a> {
    specification: &'a EventSpecification,
    category: Option<CompetitionCategory>,
}

impl Serialize for SpecificationIdentity<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut wire = serializer.serialize_struct("EventSpecification", 5)?;
        wire.serialize_field("cross_country", &self.specification.cross_country)?;
        wire.serialize_field("hurdles", &self.specification.hurdles)?;
        wire.serialize_field("implement", &self.specification.implement)?;
        wire.serialize_field("indoor_track", &self.specification.indoor_track)?;
        wire.serialize_field("category", &self.category)?;
        wire.end()
    }
}
