use super::invalid;
use census_domain::model::Evidence;
use census_domain::roster_affiliation_authority::{
    RosterAuthorityError, RosterMemberObservationV1,
};
use census_store::{StoreError, StoreResult};

pub(super) fn read(evidence: &Evidence) -> StoreResult<Option<RosterMemberObservationV1>> {
    if evidence
        .note
        .as_deref()
        .is_some_and(RosterMemberObservationV1::is_unrelated_note)
    {
        return Ok(None);
    }
    match RosterMemberObservationV1::read(evidence) {
        Ok(note) => Ok(note),
        Err(RosterAuthorityError::Decode(source)) => Err(StoreError::Decode {
            key: "roster positive row metadata".into(),
            source,
        }),
        Err(RosterAuthorityError::Invalid(detail)) => Err(invalid(detail)),
        Err(error) => Err(invalid(&error.to_string())),
    }
}

pub(super) fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
