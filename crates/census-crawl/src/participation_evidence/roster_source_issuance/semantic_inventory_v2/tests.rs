use super::*;
use crate::milesplit::SourceRowLocator;
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
#[path = "fixture.rs"]
mod fixture;
#[path = "quincy.rs"]
mod quincy;

#[test]
fn genuine_partial_preserves_all164_dispositions_and_abbott_admission() -> TestResult {
    let packet = fixture::packet()?;
    check!(eq; packet.roster().requested_url(), fixture::URL);
    check!(eq; packet.roster().acquired_at().to_rfc3339_opts(chrono::SecondsFormat::Secs, true), fixture::AT);
    let checked = packet.read()?;
    let mut grant = AttachmentReplayGrantV1::new();
    let rows = rows::inventory(checked.roster_verdict(), &mut grant)?;
    check!(eq; rows.len(), 164);
    check!(eq; rows.iter().filter(|row| matches!(row.outcome, wire::Outcome::Admitted { .. })).count(), 146);
    check!(eq; rows.iter().filter(|row| matches!(row.outcome, wire::Outcome::Rejected { reason: crate::milesplit::RosterRejectionKind::InvalidGraduationYear })).count(), 18);
    let abbott = rows
        .iter()
        .filter(|row| row.provider_id == "13749843")
        .collect::<Vec<_>>();
    let [abbott] = abbott.as_slice() else {
        return Err("exactly one Abbott row".into());
    };
    check!(matches!(abbott.outcome, wire::Outcome::Admitted { .. }));
    check!(fixture::span(abbott.row)?.contains("13749843"));
    for (ordinal, row) in rows.iter().enumerate() {
        check!(eq; usize::try_from(row.row.ordinal)?, ordinal);
        check!(!fixture::span(row.row)?.is_empty());
    }
    fixture::original_bytes()?;
    Ok(())
}

#[test]
fn rejected_unknown_overlap_duplicate_and_unfinished_refuse() -> TestResult {
    let checked = fixture::packet()?.read()?;
    for mutation in 0..5 {
        let mut verdict = checked.roster_verdict().clone();
        let RosterVerdict::Partial {
            roster,
            rejected,
            unfinished,
        } = &mut verdict
        else {
            return Err("partial fixture".into());
        };
        let overlap = roster
            .athletes
            .first()
            .ok_or("admitted member")?
            .athlete()
            .athlete_id
            .clone();
        let second = rejected.get(1).ok_or("second rejected row")?.clone();
        let first = rejected.first_mut().ok_or("rejected member")?;
        match mutation {
            0 => first.athlete_id = None,
            1 => first.athlete_id = Some(overlap),
            2 => first.athlete_id = second.athlete_id,
            3 => first.row = second.row,
            4 => *unfinished = Some(first.row),
            _ => return Err("bounded mutation".into()),
        }
        check!(rows::inventory(&verdict, &mut AttachmentReplayGrantV1::new()).is_err());
    }
    Ok(())
}

#[test]
fn genuine_compact_inventory_retains_partial_under_object_limit() -> TestResult {
    let checked = fixture::packet()?.read()?;
    let parser = super::super::manifest::compiled_roster_parser_manifest()?;
    let bytes = capture(
        &checked,
        parser.canonical_bytes(),
        b"controlled metadata",
        &mut AttachmentReplayGrantV1::new(),
    )?;
    check!(bytes.len() < LIMIT);
    check!(
        bytes.len()
            > census_domain::roster_affiliation_authority::MAX_ROSTER_AUTHORITY_RECORD_BYTES
    );
    let value: wire::Inventory = serde_json::from_slice(&bytes)?;
    check!(matches!(value.disposition, wire::Disposition::Partial));
    check!(eq; value.rows.len(), 164);
    check!(eq; value.rows.iter().filter(|row| matches!(row.outcome, wire::Outcome::Admitted { .. })).count(), 146);
    check!(eq; value.rows.iter().filter(|row| matches!(row.outcome, wire::Outcome::Rejected { reason: crate::milesplit::RosterRejectionKind::InvalidGraduationYear })).count(), 18);
    check!(eq; value.schema, SCHEMA);
    check!(eq; value.policy, POLICY);
    check!(eq; value.packet, checked.reference().as_str());
    Ok(())
}

#[path = "retained_tamper.rs"]
mod retained_tamper;

#[test]
fn ordinary_context_and_member_preimage_encoder_keeps_64k_ceiling() -> TestResult {
    let mut value = "x".repeat(64 * 1024 - 2);
    let mut grant = AttachmentReplayGrantV1::new();
    check!(eq; encode(&value, &mut grant)?.len(), 64 * 1024);
    value.push('x');
    check!(matches!(
        encode(&value, &mut grant),
        Err(census_store::StoreError::BudgetExceeded {
            what: "roster source serialization measurement",
            needed: 65537,
            available: 65536,
        })
    ));
    Ok(())
}
