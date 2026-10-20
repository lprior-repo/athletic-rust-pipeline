use super::*;

#[cfg(test)]
mod tests;

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
    if (owner || &row.id == binding.school())
        && (!owner
            || &row.id != binding.school()
            || !row.retained_conflicts.is_empty()
            || !row.same_natural_key(packet))
    {
        return Err(invalid(
            "roster binding ambiguous or conflicted school owner",
        ));
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
    if ((owner && scope) || &row.id == binding.program())
        && (!owner || !scope || &row.id != binding.program() || &row.school != binding.school())
    {
        return Err(invalid("roster binding ambiguous program scope"));
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
    phase: ScanPhase,
) -> StoreResult<bool> {
    match row {
        SourceObservation::School(actual) => {
            let same_subject = actual.namespace == expected.namespace
                && actual.source_school_id == expected.source_school_id;
            if !same_subject && actual.id != expected.id {
                return Ok(false);
            }
            if !same_subject || actual.id != expected.id {
                return Err(invalid(
                    "roster binding packet observation identity differs",
                ));
            }
            if actual == expected {
                return Ok(true);
            }
            if !actual.source_row_key.is_empty()
                && !expected.source_row_key.is_empty()
                && (phase == ScanPhase::Published
                    || actual.source_row_key != expected.source_row_key)
                && same_school_facts(actual, expected)
            {
                return Ok(false);
            }
            Err(invalid("roster binding packet observation differs"))
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

fn same_school_facts(actual: &SourceSchoolObservation, expected: &SourceSchoolObservation) -> bool {
    actual.observed_name == expected.observed_name
        && actual.city == expected.city
        && actual.state == expected.state
        && actual.association_id == expected.association_id
        && actual.district == expected.district
        && actual.official_url == expected.official_url
        && actual.athletics_net_team_id == expected.athletics_net_team_id
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
