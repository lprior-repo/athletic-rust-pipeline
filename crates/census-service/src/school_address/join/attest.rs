use census_domain::model::{SourceIdentity, SourceNamespace};
use census_domain::school_directory::{
    AttestedRecord, DirectoryError, IdentifiedKey, NcesSchoolId, PssId, SourceLabel, StateRecordId,
};
use census_domain::UsJurisdiction;

pub(super) fn owned_record(
    identity: &SourceIdentity,
    state: UsJurisdiction,
    association: Option<&str>,
) -> Result<Option<AttestedRecord>, DirectoryError> {
    let (label, key) = match &identity.namespace {
        SourceNamespace::SchoolDirectory {
            provider,
            state: owner_state,
        } if *owner_state == state => match provider.as_str() {
            "nces-ccd" => (
                SourceLabel::Ccd,
                IdentifiedKey::Nces(NcesSchoolId::parse(&identity.id)?),
            ),
            "nces-pss" => (
                SourceLabel::Pss,
                IdentifiedKey::Pss(PssId::parse(&identity.id)?),
            ),
            "state-ed" => (
                SourceLabel::StateEducationAgency { state },
                IdentifiedKey::StateRecord {
                    state,
                    id: StateRecordId::parse(&identity.id)?,
                },
            ),
            _ => return Ok(None),
        },
        SourceNamespace::AssociationSchool { association: owner }
            if association == Some(owner.as_str()) =>
        {
            (
                SourceLabel::AthleticAssociation { state },
                IdentifiedKey::StateRecord {
                    state,
                    id: StateRecordId::parse(&identity.id)?,
                },
            )
        }
        _ => return Ok(None),
    };
    Ok(Some(AttestedRecord { label, key }))
}
