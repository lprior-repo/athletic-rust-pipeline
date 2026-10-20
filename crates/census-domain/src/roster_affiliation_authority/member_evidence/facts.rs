use super::interpretation::Row;
use super::{spans, Result, SourceRowLocator};
use crate::model::{Gender, GradYear, SourceNamespace, SourceSubject, Sport};
use crate::roster_affiliation_authority::{LocallyCheckedRosterIssuanceV1, RosterAuthorityError};
use serde::Deserialize;
use serde_json::value::RawValue;
use std::borrow::Cow;

pub(super) struct DerivedFacts {
    pub(super) person: SourceSubject,
    pub(super) name: String,
    pub(super) gender: Gender,
    pub(super) grad_year: u16,
}

pub(super) fn derive(
    preimage: &RawValue,
    original: &LocallyCheckedRosterIssuanceV1,
    row: &Row<'_>,
) -> Result<DerivedFacts> {
    let fields: [&RawValue; 10] = serde_json::from_str(preimage.get())?;
    let [version, roster_name, name, gender, year, member, profile, sports, locator, current] =
        fields;
    if text(version)? != "row-local-admitted-semantics-v2" {
        return Err(RosterAuthorityError::Invalid("preimage version"));
    }
    text(roster_name)?;
    text(profile)?;
    let name = text(name)?;
    let gender: Gender = serde_json::from_str(gender.get())?;
    let year: GradYear = serde_json::from_str(year.get())?;
    let member = text(member)?;
    let sports: [bool; 3] = serde_json::from_str(sports.get())?;
    let locator: SourceRowLocator = serde_json::from_str(locator.get())?;
    if locator != row.locator || gender != original.scope().category() {
        return Err(RosterAuthorityError::Invalid(
            "preimage row or category mismatch",
        ));
    }
    provider_member(&member, row)?;
    verify_current(current, original, row, sports)?;
    let person = SourceSubject::new(SourceNamespace::MilesplitAthlete, member.into_owned())
        .map_err(|_| RosterAuthorityError::Invalid("preimage source person"))?;
    Ok(DerivedFacts {
        person,
        name: name.into_owned(),
        gender,
        grad_year: u16::try_from(year.get()).map_err(|_| RosterAuthorityError::Overflow)?,
    })
}

fn verify_current(
    raw: &RawValue,
    original: &LocallyCheckedRosterIssuanceV1,
    row: &Row<'_>,
    sports: [bool; 3],
) -> Result<()> {
    let fields: [&RawValue; 8] = serde_json::from_str(raw.get())?;
    let [owner, member, locator, section, container, heading, digest, support] = fields;
    let locator: SourceRowLocator = serde_json::from_str(locator.get())?;
    let support: [bool; 3] = serde_json::from_str(support.get())?;
    let _: [u8; 32] = serde_json::from_str(digest.get())?;
    if text(owner)? != original.scope().owner().provider_id()
        || text(member)? != row.provider_id
        || locator != row.locator
        || sports != support
        || !supports(support, original.scope().sport())
    {
        return Err(RosterAuthorityError::Invalid(
            "preimage current roster mismatch",
        ));
    }
    spans::verify(section, container, heading, locator, original)
}

fn provider_member(member: &str, row: &Row<'_>) -> Result<()> {
    if member != row.provider_id
        || member.is_empty()
        || member.len() > 128
        || !member.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(RosterAuthorityError::Invalid("preimage member mismatch"));
    }
    Ok(())
}

fn supports([indoor, outdoor, xc]: [bool; 3], sport: Sport) -> bool {
    match sport {
        Sport::IndoorTrack => indoor,
        Sport::OutdoorTrack => outdoor,
        Sport::CrossCountry => xc,
        Sport::Unknown => false,
    }
}

#[derive(Deserialize)]
struct Text<'a>(#[serde(borrow)] Cow<'a, str>);

fn text(raw: &RawValue) -> Result<Cow<'_, str>> {
    let Text(value) = serde_json::from_str(raw.get())?;
    Ok(value)
}
