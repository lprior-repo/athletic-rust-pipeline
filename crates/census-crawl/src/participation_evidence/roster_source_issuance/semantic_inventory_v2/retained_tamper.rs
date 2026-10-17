use super::*;
use crate::participation_evidence::roster_source_issuance::{historical::CheckedOriginalRosterPacket, manifest, source};
use source::tests::integration::setup;
use census_domain::model::{CanonicalSchool, CanonicalTeam, Gender, SchoolYear, SourceIdentity, SourceNamespace, SourceObservation, Sport};
use census_domain::roster_affiliation_authority::RosterAuthorityObjectRefV1;
use census_store::{RosterIssuanceInspectionRequestV1, Store, Table, RetainedCaptureRootSchemaV1};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

#[derive(Clone, Copy)]
enum Mutation { DropRejected, ReclassifyRejected, AlterAbbott, Reorder }

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope<'a> {
    schema: &'a str,
    #[serde(borrow)]
    original_capture_interpretation: &'a RawValue,
    #[serde(borrow)]
    semantic_inventory: &'a RawValue,
}

#[test]
fn genuine_retained_disposition_tampering_refuses_after_valid_replay_and_rehashed_envelope() -> TestResult {
    let (_directory, store) = setup::enrolled_store()?;
    let packet = fixture::packet()?;
    let original_packet = packet.encode()?;
    let (school, program, request) = install(&store, &packet)?;
    for mutation in [Mutation::DropRejected, Mutation::ReclassifyRejected, Mutation::AlterAbbott, Mutation::Reorder] {
        let selected = RosterIssuanceInspectionRequestV1 { root: request.root.clone(), school: school.id.clone(), program: program.id.clone() };
        let mut inspection = store.inspect_roster_source_issuance_v1(selected, AttachmentReplayGrantV1::new())?;
        let (evidence, grant) = inspection.source_parts()?;
        let proposal = setup::actual_proposal(&packet, &school, &program, evidence, grant)?;
        let parser = manifest::compiled_roster_parser_manifest()?;
        let checked_parser = manifest::assess_retained_roster_parser_manifest(parser.canonical_bytes(), &parser.parser_claim()?, grant)?;
        let capture_objects = crate::participation_evidence::roster_source_issuance::prepare_retained_roster_capture_objects_v3(evidence, &checked_parser, grant)?;
        let metadata = capture_objects.metadata_bytes(grant)?.to_vec();
        let original = capture_objects.interpretation_bytes(grant)?.to_vec();
        let envelope: Envelope<'_> = serde_json::from_slice(&original)?;
        let witness = CheckedOriginalRosterPacket::read_for_test(evidence, &proposal, grant)?;
        let baseline = replay::compare(&witness, parser.retained(), &metadata, envelope.semantic_inventory.get().as_bytes(), grant)?;
        check!(eq; baseline.reference(), &packet.reference()?);
        let RosterVerdict::Partial { roster, rejected, unfinished } = baseline.roster_verdict() else { return Err("positive genuine replay must remain partial".into()); };
        check!(eq; roster.athletes.len(), 146);
        check!(eq; rejected.len(), 18);
        check!(unfinished.is_none());
        grant.ensure_available()?;
        let changed = changed_envelope(&envelope, mutation)?;
        let reference: RosterAuthorityObjectRefV1 = serde_json::from_value(serde_json::json!({"digest":hash(&changed), "bytes":changed.len()}))?;
        source::source_reference(&changed, &reference, grant)?;
        check!(hash(&changed) != hash(&original));
        let changed: Envelope<'_> = serde_json::from_slice(&changed)?;
        check!(eq; changed.original_capture_interpretation.get(), envelope.original_capture_interpretation.get());
        let result = replay::compare(&witness, parser.retained(), &metadata, changed.semantic_inventory.get().as_bytes(), grant);
        check!(matches!(result, Err(crate::CrawlError::Store(census_store::StoreError::Refused { detail })) if detail == "row-local retained inventory differs from original capture"));
        check!(grant.ensure_available().is_err());
        check!(replay::compare(&witness, parser.retained(), &metadata, envelope.semantic_inventory.get().as_bytes(), grant).is_err());
        check!(eq; packet.encode()?, original_packet);
    }
    fixture::original_bytes()?;
    Ok(())
}

fn changed_envelope(original: &Envelope<'_>, mutation: Mutation) -> TestResult<Vec<u8>> {
    let mut inventory: wire::Inventory = serde_json::from_str(original.semantic_inventory.get())?;
    match mutation {
        Mutation::DropRejected => {
            let ordinal = inventory.rows.iter().find(|row| matches!(row.outcome, wire::Outcome::Rejected { .. })).ok_or("rejected row")?.row.ordinal;
            inventory.rows.retain(|row| row.row.ordinal != ordinal);
        }
        Mutation::ReclassifyRejected => {
            inventory.rows.iter_mut().find(|row| matches!(row.outcome, wire::Outcome::Rejected { .. })).ok_or("rejected row")?.outcome = wire::Outcome::Admitted { semantic_digest: "0".repeat(64) };
        }
        Mutation::AlterAbbott => {
            inventory.rows.iter_mut().find(|row| row.provider_id == "13749843").ok_or("Abbott row")?.outcome = wire::Outcome::Admitted { semantic_digest: "0".repeat(64) };
        }
        Mutation::Reorder => inventory.rows.reverse(),
    }
    let bytes = serde_json::to_vec(&inventory)?;
    let changed = Envelope { schema: original.schema, original_capture_interpretation: original.original_capture_interpretation, semantic_inventory: serde_json::from_slice(&bytes)? };
    Ok(serde_json::to_vec(&changed)?)
}

fn install(store: &Store, packet: &crate::participation_evidence::provider_school::ProviderSchoolPacketV1) -> TestResult<(CanonicalSchool, CanonicalTeam, RosterIssuanceInspectionRequestV1)> {
    let checked = packet.read()?;
    let school = checked.school().clone();
    let year = SchoolYear::new(2026).ok_or("fixture year")?;
    let program = CanonicalTeam {
        id: CanonicalTeam::mint(&school.id, Sport::IndoorTrack, Gender::Boys, year),
        school: school.id.clone(), sport: Sport::IndoorTrack, gender: Gender::Boys, school_year: year,
        level: None, source_identities: vec![SourceIdentity::new(SourceNamespace::MilesplitTeam, "12439")],
        evidence: Vec::new(), retained_conflicts: Vec::new(),
    };
    store.append(Table::Schools, &school)?;
    store.append(Table::Teams, &program)?;
    store.append(Table::SourceObservations, &SourceObservation::School(checked.observation().clone()))?;
    let schema = RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1;
    let reference = packet.reference()?;
    store.journal_done(schema.phase(), reference.as_str(), &serde_json::to_value(packet)?)?;
    let held = store.snapshot().read_retained_capture_root_v1(schema, reference.as_str(), &mut AttachmentReplayGrantV1::new())?.ok_or("retained original packet")?;
    let request = RosterIssuanceInspectionRequestV1 { root: held.root().clone(), school: school.id.clone(), program: program.id.clone() };
    Ok((school, program, request))
}
