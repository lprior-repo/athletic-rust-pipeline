use super::*;
use census_crawl::milesplit::SourceRowLocator as CrawlLocator;
use census_domain::roster_affiliation_authority::{
    check_roster_member_witness_v1, RosterAuthorityError, RosterAuthorityLimitsV1,
};

fn member_bundle(
    fixture: &fixture::Fixture,
    capture: &fixture::Capture,
) -> TestResult<(
    LocallyCheckedRosterIssuanceV1,
    Vec<u8>,
    Vec<u8>,
    CrawlLocator,
)> {
    let schema = census_store::RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1;
    let retained = fixture
        .store
        .snapshot()
        .read_retained_capture_root_v1(
            schema,
            capture.packet.reference()?.as_str(),
            &mut AttachmentReplayGrantV1::new(),
        )?
        .ok_or("controlled retained packet")?;
    let request = census_store::RosterIssuanceInspectionRequestV1 {
        root: retained.root().clone(),
        school: capture.school.id.clone(),
        program: capture.program.id.clone(),
    };
    let mut inspection = fixture
        .store
        .inspect_roster_source_issuance_v1(request, AttachmentReplayGrantV1::new())?;
    let (evidence, grant) = inspection.source_parts()?;
    let proposal =
        super::proposal_revision(capture, evidence, grant, super::CaptureRevision::RowLocalV3)?;
    let parser = census_crawl::participation_evidence::roster_source_issuance::manifest::compiled_roster_parser_manifest()?;
    let contracts = census_crawl::participation_evidence::roster_source_issuance::policies::compiled_roster_interpretation_contracts_v1();
    let assessment = census_crawl::participation_evidence::roster_source_issuance::assess_retained_roster_source_v3(
        evidence,
        parser.canonical_bytes(),
        &contracts,
        &proposal,
        grant,
    )?;
    let locator = {
        let checked = assessment.checked_packet(grant)?;
        let roster = checked
            .roster_verdict()
            .roster()
            .ok_or("controlled parsed roster")?;
        let member = roster.athletes.first().ok_or("controlled member")?;
        member.locator()
    };
    let witness = assessment.member_semantic_witness_v1(locator, grant)?;
    let interpretation = {
        let objects = assessment.capture_objects(grant)?;
        objects.interpretation_bytes(grant)?.to_vec()
    };
    Ok((proposal, interpretation, witness, locator))
}

fn limits() -> TestResult<RosterAuthorityLimitsV1> {
    Ok(RosterAuthorityLimitsV1::new(64 * 1024, 8 * 1024 * 1024)?)
}

fn expected_person() -> TestResult<SourceSubject> {
    Ok(SourceSubject::new(
        SourceNamespace::MilesplitAthlete,
        "77".to_string(),
    )?)
}

#[test]
fn source_a_member_evidence_matches_exact_original_control() -> TestResult {
    let fixture = fixture::Fixture::new()?;
    let capture = fixture::install(&fixture, "11111", "77", Sport::OutdoorTrack, true)?;
    let (proposal, interpretation, witness, locator) = member_bundle(&fixture, &capture)?;
    let checked = check_roster_member_witness_v1(&proposal, &interpretation, &witness, limits()?)?;
    let person = expected_person()?;
    check!(eq; checked.source_person(), &person);
    check!(eq; checked.name(), "Controlled Runner");
    check!(eq; checked.gender(), Gender::Boys);
    check!(eq; checked.grad_year(), Some(2027));
    let expected_claim = census_domain::model::serialized_digest(&(&proposal, &person, &locator))?;
    check!(eq; checked.claim_id(), expected_claim.as_str());
    check!(eq; checked.row_locator().ordinal(), locator.ordinal);
    check!(eq; checked.row_locator().byte_offset(), locator.byte_offset);
    check!(eq; checked.row_locator().byte_length(), locator.byte_length);
    check!(eq; checked.row_provider_id(), "77");
    Ok(())
}

#[test]
fn foreign_interpretation_with_original_proposal_is_invalid() -> TestResult {
    let fixture_a = fixture::Fixture::new()?;
    let capture_a = fixture::install(&fixture_a, "11111", "77", Sport::OutdoorTrack, true)?;
    let (proposal_a, _, _, _) = member_bundle(&fixture_a, &capture_a)?;
    let fixture_b = fixture::Fixture::new()?;
    let capture_b = fixture::install(&fixture_b, "22222", "77", Sport::OutdoorTrack, true)?;
    let (proposal_b, interpretation_b, witness_b, _) = member_bundle(&fixture_b, &capture_b)?;
    let valid_b =
        check_roster_member_witness_v1(&proposal_b, &interpretation_b, &witness_b, limits()?)?;
    check!(eq; valid_b.source_person(), &expected_person()?);
    check!(ne;
        proposal_a.scope().owner().provider_id(),
        capture_b
            .school
            .source_identities
            .iter()
            .find(|identity| identity.namespace == SourceNamespace::MilesplitSchool)
            .ok_or("controlled owner")?
            .id
    );
    let result =
        check_roster_member_witness_v1(&proposal_a, &interpretation_b, &witness_b, limits()?);
    check!(matches!(result, Err(RosterAuthorityError::Invalid(_))));
    Ok(())
}

#[test]
fn foreign_witness_with_actual_interpretation_is_invalid() -> TestResult {
    let fixture_a = fixture::Fixture::new()?;
    let capture_a = fixture::install(&fixture_a, "11111", "77", Sport::OutdoorTrack, true)?;
    let (proposal_a, interpretation_a, _, _) = member_bundle(&fixture_a, &capture_a)?;
    let fixture_b = fixture::Fixture::new()?;
    let capture_b = fixture::install(&fixture_b, "22222", "77", Sport::OutdoorTrack, true)?;
    let (_, _, witness_b, _) = member_bundle(&fixture_b, &capture_b)?;
    let result =
        check_roster_member_witness_v1(&proposal_a, &interpretation_a, &witness_b, limits()?);
    check!(matches!(result, Err(RosterAuthorityError::Invalid(_))));
    Ok(())
}

#[test]
fn tampered_preimage_with_original_envelope_is_invalid() -> TestResult {
    let fixture = fixture::Fixture::new()?;
    let capture = fixture::install(&fixture, "11111", "77", Sport::OutdoorTrack, true)?;
    let (proposal, interpretation, witness, _) = member_bundle(&fixture, &capture)?;
    let mut witness_value: Value = serde_json::from_slice(&witness)?;
    let preimage = witness_value
        .get_mut("semantic_preimage")
        .ok_or("controlled witness preimage")?;
    let array = preimage.as_array_mut().ok_or("controlled preimage array")?;
    let name_slot = array.get_mut(2).ok_or("controlled name slot")?;
    *name_slot = Value::String("Tampered Name".to_string());
    let tampered = serde_json::to_vec(&witness_value)?;
    let result = check_roster_member_witness_v1(&proposal, &interpretation, &tampered, limits()?);
    check!(matches!(result, Err(RosterAuthorityError::Invalid(_))));
    Ok(())
}

#[test]
fn changed_interpretation_digest_cannot_reuse_original_proposal() -> TestResult {
    let fixture = fixture::Fixture::new()?;
    let capture = fixture::install(&fixture, "11111", "77", Sport::OutdoorTrack, true)?;
    let (proposal, interpretation, witness, _) = member_bundle(&fixture, &capture)?;
    let mut value: Value = serde_json::from_slice(&interpretation)?;
    let metadata = value
        .pointer_mut("/semantic_inventory/metadata")
        .ok_or("original interpretation metadata reference")?;
    let original = metadata.as_str().ok_or("metadata digest")?;
    let mut changed = original.to_string();
    changed.replace_range(..1, if original.starts_with('0') { "1" } else { "0" });
    *metadata = Value::String(changed);
    let tampered = serde_json::to_vec(&value)?;
    check!(eq; tampered.len(), interpretation.len());
    let result = check_roster_member_witness_v1(&proposal, &tampered, &witness, limits()?);
    check!(matches!(result, Err(RosterAuthorityError::Invalid(_))));
    Ok(())
}
#[test]
fn consistent_out_of_range_locators_reach_span_bound() -> TestResult {
    let fixture = fixture::Fixture::new()?;
    let capture = fixture::install(&fixture, "11111", "77", Sport::OutdoorTrack, true)?;
    let (proposal, interpretation, witness, locator) = member_bundle(&fixture, &capture)?;
    let far = 4 * 1024 * 1024;
    let mut witness_value: Value = serde_json::from_slice(&witness)?;
    {
        let array = witness_value
            .get_mut("semantic_preimage")
            .ok_or("controlled witness preimage")?
            .as_array_mut()
            .ok_or("controlled preimage array")?;
        let outer = array.get_mut(8).ok_or("controlled locator slot")?;
        let outer_offset = outer
            .get_mut("byte_offset")
            .ok_or("controlled offset slot")?;
        *outer_offset = Value::from(far);
        let current = array.get_mut(9).ok_or("controlled current slot")?;
        let items = current.as_array_mut().ok_or("controlled current array")?;
        let inner = items.get_mut(2).ok_or("controlled inner slot")?;
        let inner_offset = inner
            .get_mut("byte_offset")
            .ok_or("controlled inner offset")?;
        *inner_offset = Value::from(far);
    }
    let mutated_preimage = witness_value
        .get("semantic_preimage")
        .ok_or("controlled mutated preimage")?;
    let mutated_bytes = serde_json::to_vec(&mutated_preimage)?;
    let mutated_digest = super::super::hash(&mutated_bytes);
    let tampered_witness = serde_json::to_vec(&witness_value)?;
    let mut interpretation_value: Value = serde_json::from_slice(&interpretation)?;
    {
        let rows = if let Some(rows) = interpretation_value
            .get_mut("rows")
            .and_then(Value::as_array_mut)
        {
            rows
        } else if let Some(inventory) = interpretation_value.get_mut("semantic_inventory") {
            inventory
                .get_mut("rows")
                .and_then(Value::as_array_mut)
                .ok_or("controlled inventory rows")?
        } else {
            return Err("controlled interpretation rows".into());
        };
        let row = rows.get_mut(0).ok_or("controlled row")?;
        let row_locator = row.get_mut("row").ok_or("controlled row locator")?;
        let row_offset = row_locator
            .get_mut("byte_offset")
            .ok_or("controlled row offset")?;
        *row_offset = Value::from(far);
        check!(eq; row.get("provider_id").and_then(Value::as_str).ok_or("controlled provider")?, "77");
        let outcome = row.get_mut("outcome").ok_or("controlled outcome")?;
        let digest_slot = outcome
            .get_mut("semantic_digest")
            .ok_or("controlled digest slot")?;
        *digest_slot = Value::String(mutated_digest);
    }
    let tampered_interpretation = serde_json::to_vec(&interpretation_value)?;
    check!(eq; locator.ordinal, 0);
    let result = check_roster_member_witness_v1(
        &proposal,
        &tampered_interpretation,
        &tampered_witness,
        limits()?,
    );
    check!(matches!(result, Err(RosterAuthorityError::Invalid(_))));
    Ok(())
}
