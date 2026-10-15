use super::Inputs;
use crate::{StoreError, StoreResult};
use census_domain::model::{AppliedAthleteIdentity, SourceIdentity};

pub(in crate::public_identity) fn source(
    inputs: &Inputs,
    decision: &AppliedAthleteIdentity,
) -> StoreResult<SourceIdentity> {
    let [member] = decision.members.as_slice() else {
        return Err(invalid("unsupported retained authority membership"));
    };
    let (accepted, source) =
        super::super::authority::accept_inputs(inputs, &member.subject, &decision.observed_at)?;
    if accepted.record() != decision {
        return Err(invalid("retained authority decision does not replay"));
    }
    Ok(source)
}

pub(in crate::public_identity) fn reviewed(
    inputs: &Inputs,
    decision: &AppliedAthleteIdentity,
) -> StoreResult<()> {
    let case_id = decision
        .case_id
        .as_deref()
        .ok_or_else(|| invalid("reviewed authority case missing"))?;
    let accepted =
        super::super::authority::accept_reviewed(inputs, case_id, &decision.observed_at)?;
    if accepted.record() != decision {
        return Err(invalid(
            "retained reviewed authority decision does not replay",
        ));
    }
    Ok(())
}
fn invalid(detail: &str) -> StoreError {
    StoreError::Refused {
        detail: detail.into(),
    }
}
