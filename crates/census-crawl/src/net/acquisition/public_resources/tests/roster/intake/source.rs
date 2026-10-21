use super::*;
use crate::participation_evidence::provider_school::CheckedProviderSchoolPacketV1;

pub(super) fn verify_school(checked: &CheckedProviderSchoolPacketV1) -> TestResult {
    let school = checked.school();
    check!(eq; school.name, "Detroit Catholic Central");
    check!(eq; school.city.as_deref(), Some("Novi"));
    let [identity] = school.source_identities.as_slice() else {
        return Err("one native school identity required".into());
    };
    check!(eq; identity.namespace, SourceNamespace::MilesplitSchool);
    check!(eq; identity.id, TEAM);
    check!(eq; identity.url.as_deref(), Some("https://mi.milesplit.com/teams/12439-detroit-catholic-central"));
    check!(eq; school.evidence.len(), 2);
    check!(eq; checked.observation().source_school_id, TEAM);
    check!(eq; checked.directory_proof().row().ordinal, 241);
    check!(eq; checked.directory_proof().row().byte_offset, 91718);
    check!(eq; checked.directory_proof().row().byte_length, 271);
    verify_roster(checked)
}

fn verify_roster(checked: &CheckedProviderSchoolPacketV1) -> TestResult {
    let crate::milesplit::RosterVerdict::Partial {
        roster,
        rejected,
        unfinished,
    } = checked.roster_verdict()
    else {
        return Err("genuine roster retains invalid graduation rows".into());
    };
    check!(eq; roster.athletes.len(), 146);
    check!(eq; rejected.len(), 18);
    check!(unfinished.is_none());
    let mut cohort = 0usize;
    for member in &roster.athletes {
        if member.athlete().grad_year == GradYear::CO2027 {
            cohort = cohort
                .checked_add(1)
                .ok_or("source cohort count overflow")?;
            verify_source_row(member)?;
        }
    }
    check!(eq; cohort, 45);
    Ok(())
}

fn verify_source_row(member: &crate::milesplit::LocatedRosterAthlete) -> TestResult {
    let row = member.locator();
    let end = row
        .byte_offset
        .checked_add(row.byte_length)
        .ok_or("source row overflow")?;
    let span = ROSTER
        .get(row.byte_offset..end)
        .ok_or("source row bounds")?;
    let span = std::str::from_utf8(span)?;
    check!(span.contains(&member.athlete().profile_url));
    check!(span.contains("2027"));
    Ok(())
}
