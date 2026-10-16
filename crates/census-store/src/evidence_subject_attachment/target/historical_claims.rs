use super::*;
use serde::Deserialize;
use serde_json::value::RawValue;

#[derive(Deserialize)]
#[serde(
    tag = "kind",
    content = "claim",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum Claim<'a> {
    #[serde(borrow)]
    Binding(Binding<'a>),
    #[serde(borrow)]
    Membership(Membership<'a>),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding<'a> {
    #[serde(borrow)]
    id: &'a str,
    #[serde(borrow)]
    source: &'a RawValue,
    candidate: AthleteCandidateId,
    #[serde(borrow)]
    effective: &'a RawValue,
    #[serde(borrow)]
    supersedes: Option<&'a str>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Membership<'a> {
    #[serde(borrow)]
    id: &'a str,
    candidate: AthleteCandidateId,
    #[serde(borrow)]
    anchor_binding_id: &'a str,
    #[serde(borrow)]
    source: &'a RawValue,
}

pub(super) fn validate(raw: &[&RawValue], grant: &mut AttachmentReplayGrantV1) -> StoreResult<()> {
    let mut previous = None;
    for raw in raw {
        grant.charge_steps(1).map_err(Error::Domain)?;
        let claim: Claim<'_> =
            serde_json::from_str(raw.get()).map_err(|source| Error::Domain(source.into()))?;
        let (id, candidate) = match &claim {
            Claim::Binding(binding) => {
                let _retained = (binding.source, binding.effective, binding.supersedes);
                (binding.id, &binding.candidate)
            }
            Claim::Membership(membership) => {
                let _retained = (membership.source, membership.anchor_binding_id);
                (membership.id, &membership.candidate)
            }
        };
        if id.is_empty()
            || id.len() > ReplayLimits::SMALL.text_bytes
            || !candidate.validate("ath")
            || previous.is_some_and(|prior| prior >= id)
        {
            return Err(Error::Corrupt("historical claim order or identifier").into());
        }
        previous = Some(id);
    }
    Ok(())
}
