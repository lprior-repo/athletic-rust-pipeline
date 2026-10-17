use super::*;
use crate::participation_evidence::roster_source_issuance::manifest::compiled_roster_parser_manifest;
use crate::participation_evidence::roster_source_issuance::assess_existing_roster_binding;
use census_domain::model::{CanonicalSchool, CanonicalTeam, Gender, SchoolYear, SourceIdentity, SourceObservation, Sport};
use census_domain::roster_affiliation_authority::RosterAuthorityTextV1;
use census_store::{
    RosterEnrollmentReviewV1, RosterIssuanceInspectionRequestV1, RosterReviewActorV1,
    Store, Table,
};

const SMALL_DIRECTORY: &str = concat!(
    "<html><head><meta name=\"application-name\" content=\"MileSplit Michigan\">",
    "<link rel=\"canonical\" href=\"https://mi.milesplit.com/teams\"></head><body><table>",
    "<tr><td><a href=\"https://mi.milesplit.com/teams/12439-detroit-catholic-central\">",
    "Controlled School</a></td><td>Novi, MI, USA</td></tr></table></body></html>"
);
const SMALL_ROSTER: &str = concat!(
    "<html><head><link rel=\"canonical\" href=\"https://mi.milesplit.com/teams/12439-detroit-catholic-central/roster\">",
    "</head><body><section><h4>Current Roster</h4><ul id=\"rosterDataset\">",
    "<li class=\"athlete-row data-row\"><a href=\"https://mi.milesplit.com/athletes/42-controlled\">",
    "Runner, Controlled</a><div class=\"column-gender\">m</div>",
    "<div class=\"column-grad-year\">2027</div><div class=\"data-point\" data-season-id=\"1\">",
    "<svg class=\"icon icon-yes\"></svg></div></li></ul></section></body></html>"
);

fn enrolled_store() -> TestResult<(tempfile::TempDir, Store)> {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("controlled-roster");
    Store::create_public_identity_qualification_v3(&root)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    let inputs = store.inspect_roster_source_enrollment_v1()?;
    let prepared = store.prepare_roster_source_enrollment_v1(inputs, RosterEnrollmentReviewV1 {
        operation: RosterAuthorityTextV1::parse("controlled-crawl-inspection")?,
        actor: RosterReviewActorV1::LocalOperator,
        reviewed_at: DateTime::parse_from_rfc3339("2026-10-16T00:49:43Z")?.to_utc(),
        rationale: RosterAuthorityTextV1::parse("Controlled parser and grant integration test")?,
    })?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    store.apply_roster_source_enrollment_v1(prepared, &approved, &digest)?;
    Ok((directory, store))
}

fn install(store: &Store) -> TestResult<(ProviderSchoolPacketV1, CanonicalSchool, CanonicalTeam, RosterIssuanceInspectionRequestV1)> {
    let packet = ProviderSchoolPacketV1::new(
        retained("https://mi.milesplit.com/teams", SMALL_DIRECTORY)?, retained(URL, SMALL_ROSTER)?)?;
    let checked = packet.read()?;
    let school = checked.school().clone();
    let year = SchoolYear::new(2026).ok_or("year")?;
    let program = CanonicalTeam {
        id: CanonicalTeam::mint(&school.id, Sport::IndoorTrack, Gender::Boys, year),
        school: school.id.clone(), sport: Sport::IndoorTrack, gender: Gender::Boys,
        school_year: year, level: None, source_identities: vec![
            SourceIdentity::new(SourceNamespace::MilesplitTeam, "12439")],
        evidence: Vec::new(), retained_conflicts: Vec::new(),
    };
    store.append(Table::Schools, &school)?;
    store.append(Table::Teams, &program)?;
    store.append(Table::SourceObservations, &SourceObservation::School(checked.observation().clone()))?;
    let schema = RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1;
    let reference = packet.reference()?;
    store.journal_done(schema.phase(), reference.as_str(), &serde_json::to_value(&packet)?)?;
    let held = store.snapshot().read_retained_capture_root_v1(schema, reference.as_str(),
        &mut AttachmentReplayGrantV1::new())?.ok_or("root missing")?;
    let request = RosterIssuanceInspectionRequestV1 {
        root: held.root().clone(), school: school.id.clone(), program: program.id.clone(),
    };
    Ok((packet, school, program, request))
}

fn object(bytes: &[u8]) -> Value {
    json!({"digest":hash(bytes), "bytes":bytes.len()})
}

fn actual_proposal(
    packet: &ProviderSchoolPacketV1,
    school: &CanonicalSchool,
    program: &CanonicalTeam,
    evidence: &RosterSourceEvidenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult<LocallyCheckedRosterIssuanceV1> {
    let mut value = proposal(packet, SMALL_ROSTER)?;
    let compiled = compiled_roster_parser_manifest()?;
    for (path, replacement) in [
        ("/parser", serde_json::to_value(compiled.parser_claim()?)?),
        ("/scope/sport", json!("indoor_track")), ("/binding/sport", json!("indoor_track")),
        ("/binding/school", json!(school.id)), ("/binding/program", json!(program.id)),
        ("/binding/store", json!(evidence.store_identity())),
        ("/binding/snapshot_revision", json!(evidence.snapshot_sequence())),
        ("/capture/retained_source_admission", object(&evidence.retained_source_admission_manifest(grant)?)),
        ("/capture/verification_grant", object(&evidence.verification_grant_manifest(grant)?)),
        ("/binding/school_projection", object(&serde_json::to_vec(school)?)),
        ("/binding/program_projection", object(&serde_json::to_vec(program)?)),
        ("/binding/physical_projection_inventory", object(&evidence.physical_projection_manifest(grant)?)),
    ] {
        *value.pointer_mut(path).ok_or("proposal field missing")? = replacement;
    }
    decode(&value)
}

#[test]
fn controlled_public_inspection_binds_exact_rows_and_refuses_foreign_grant() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    let (packet, school, program, request) = install(&store)?;
    let mut inspection = store.inspect_roster_source_issuance_v1(request, AttachmentReplayGrantV1::new())?;
    let (evidence, grant) = inspection.source_parts()?;
    let proposal = actual_proposal(&packet, &school, &program, evidence, grant)?;
    let compiled = compiled_roster_parser_manifest()?;
    let usage = grant.usage();
    let mut foreign = AttachmentReplayGrantV1::new();
    foreign.charge_bytes(usage.encoded_bytes())?;
    foreign.charge_steps(usage.traversal_steps())?;
    let original = std::mem::replace(grant, foreign);
    check!(assess_retained_roster_source(evidence, compiled.canonical_bytes(), &proposal, grant).is_err());
    *grant = original;
    let source = assess_retained_roster_source(evidence, compiled.canonical_bytes(), &proposal, grant)?;
    let binding = assess_existing_roster_binding(&source, grant)?;
    check!(eq; binding.school(grant)?.id, school.id);
    check!(eq; binding.program(grant)?.id, program.id);
    check!(std::ptr::eq(binding.evidence(grant)?, evidence));
    Ok(())
}

#[test]
fn controlled_inspection_rejects_substituted_parser_and_context_references() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    let (packet, school, program, request) = install(&store)?;
    for path in ["/capture/retained_source_admission/digest", "/capture/verification_grant/digest",
        "/parser/artifact/digest", "/parser/schema/digest", "/parser/configuration/digest", "/parser/dependencies/digest"] {
        let root = request.root.clone();
        let selected = RosterIssuanceInspectionRequestV1 {
            root, school: school.id.clone(), program: program.id.clone(),
        };
        let mut inspection = store.inspect_roster_source_issuance_v1(selected, AttachmentReplayGrantV1::new())?;
        let (evidence, grant) = inspection.source_parts()?;
        let proposal = actual_proposal(&packet, &school, &program, evidence, grant)?;
    let compiled = compiled_roster_parser_manifest()?;
        let mut value = serde_json::to_value(&proposal)?;
        *value.pointer_mut(path).ok_or("context reference absent")? = json!("d".repeat(64));
        let changed = decode(&value)?;
        check!(assess_retained_roster_source(evidence, compiled.canonical_bytes(), &changed, grant).is_err());
        check!(grant.ensure_available().is_err());
    }
    Ok(())
}

#[test]
fn controlled_inspection_refuses_wrong_retained_parser_manifest() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    let (packet, school, program, request) = install(&store)?;
    let mut inspection = store.inspect_roster_source_issuance_v1(request, AttachmentReplayGrantV1::new())?;
    let (evidence, grant) = inspection.source_parts()?;
    let proposal = actual_proposal(&packet, &school, &program, evidence, grant)?;
    check!(assess_retained_roster_source(evidence, b"{}", &proposal, grant).is_err());
    check!(grant.ensure_available().is_err());
    Ok(())
}

#[test]
fn controlled_inspection_refuses_changed_requested_school_or_program() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    let (packet, school, program, request) = install(&store)?;
    for change_school in [true, false] {
        let selected = RosterIssuanceInspectionRequestV1 {
            root: request.root.clone(),
            school: if change_school { census_domain::model::SchoolId::mint("sch", &["other"]) } else { school.id.clone() },
            program: if change_school { program.id.clone() } else { census_domain::model::TeamId::mint("team", &["other"]) },
        };
        let mut inspection = store.inspect_roster_source_issuance_v1(selected, AttachmentReplayGrantV1::new())?;
        let (evidence, grant) = inspection.source_parts()?;
        let proposal = actual_proposal(&packet, &school, &program, evidence, grant)?;
        let compiled = compiled_roster_parser_manifest()?;
        check!(assess_retained_roster_source(evidence, compiled.canonical_bytes(), &proposal, grant).is_err());
        check!(grant.ensure_available().is_err());
    }
    Ok(())
}
