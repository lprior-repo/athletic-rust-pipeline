use super::*;

pub(super) fn school_row(
    row: &CanonicalSchool,
    binding: &RosterExistingBindingClaimV1,
    packet: &CanonicalSchool,
) -> StoreResult<()> {
    let owner = owns(
        &row.source_identities,
        SourceNamespace::MilesplitSchool,
        binding.school_source().provider_id(),
    );
    if owner || &row.id == binding.school() {
        if !owner
            || &row.id != binding.school()
            || !row.retained_conflicts.is_empty()
            || !row.same_natural_key(packet)
        {
            return Err(invalid(
                "roster binding ambiguous or conflicted school owner",
            ));
        }
    }
    Ok(())
}

pub(super) fn team_row(
    row: &CanonicalTeam,
    binding: &RosterExistingBindingClaimV1,
) -> StoreResult<()> {
    let owner = owns(
        &row.source_identities,
        SourceNamespace::MilesplitTeam,
        binding.program_source().provider_id(),
    );
    let scope = row.sport == binding.sport()
        && row.gender == binding.category()
        && row.school_year == binding.school_year();
    if owner && (&row.school != binding.school() || !row.retained_conflicts.is_empty()) {
        return Err(invalid("roster binding conflicted program owner"));
    }
    if (owner && scope) || &row.id == binding.program() {
        if !owner || !scope || &row.id != binding.program() || &row.school != binding.school() {
            return Err(invalid("roster binding ambiguous program scope"));
        }
    }
    Ok(())
}

pub(super) fn no_global_conflict(_: &RetainedConflict) -> StoreResult<()> {
    Err(invalid(
        "roster binding requires all global conflicts absent",
    ))
}

pub(super) fn owns(identities: &[SourceIdentity], namespace: SourceNamespace, id: &str) -> bool {
    identities
        .iter()
        .any(|identity| identity.namespace == namespace && identity.id == id)
}

pub(super) fn observation(
    row: &SourceObservation,
    expected: &SourceSchoolObservation,
) -> StoreResult<bool> {
    match row {
        SourceObservation::School(actual) => {
            let subject = actual.namespace == expected.namespace
                && actual.source_school_id == expected.source_school_id;
            if subject || actual.id == expected.id {
                if actual != expected {
                    return Err(invalid("roster binding packet observation differs"));
                }
                return Ok(true);
            }
            Ok(false)
        }
        SourceObservation::Athlete(_) => {
            if row.id() == expected.id {
                return Err(invalid(
                    "roster binding school observation ID has foreign object",
                ));
            }
            Ok(false)
        }
    }
}

pub(super) fn select<T: Entity>(selected: &mut Option<T>, row: T, id: &str) -> StoreResult<()> {
    if row.entity_id() == id {
        if selected.is_some() {
            return Err(invalid("roster binding duplicate published identity"));
        }
        *selected = Some(row);
    }
    Ok(())
}
