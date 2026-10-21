use super::*;
use crate::participation_evidence::roster_source_issuance::manifest::compiled_roster_parser_manifest;
use crate::participation_evidence::roster_source_issuance::policies::compiled_roster_interpretation_contracts_v1;
use census_domain::model::{
    CanonicalSchool, CanonicalTeam, Gender, SchoolYear, SourceIdentity, SourceObservation, Sport,
};
use census_domain::roster_affiliation_authority::RosterAuthorityTextV1;
use census_store::{
    RosterEnrollmentReviewV1, RosterIssuanceInspectionRequestV1, RosterReviewActorV1, Store, Table,
};

pub(in crate::participation_evidence::roster_source_issuance) const SMALL_DIRECTORY: &str = concat!(
    "<html><head><meta name=\"application-name\" content=\"MileSplit Michigan\">",
    "<link rel=\"canonical\" href=\"https://mi.milesplit.com/teams\"></head><body><table>",
    "<tr><td><a href=\"https://mi.milesplit.com/teams/12439-detroit-catholic-central\">",
    "Controlled School</a></td><td>Novi, MI, USA</td></tr></table></body></html>"
);
pub(in crate::participation_evidence::roster_source_issuance) const SMALL_ROSTER: &str = concat!(
    "<html><head><link rel=\"canonical\" href=\"https://mi.milesplit.com/teams/12439-detroit-catholic-central/roster\">",
    "</head><body><section><h4>Current Roster</h4><ul id=\"rosterDataset\">",
    "<li class=\"athlete-row data-row\"><a href=\"https://mi.milesplit.com/athletes/42-controlled\">",
    "Runner, Controlled</a><div class=\"column-gender\">m</div>",
    "<div class=\"column-grad-year\">2027</div><div class=\"data-point\" data-season-id=\"1\">",
    "<svg class=\"icon icon-yes\"></svg></div></li></ul></section></body></html>"
);

pub(in crate::participation_evidence::roster_source_issuance) fn enrolled_store(
) -> TestResult<(tempfile::TempDir, Store)> {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("controlled-roster");
    Store::create_public_identity_qualification_v3(&root)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    let inputs = store.inspect_roster_source_enrollment_v1()?;
    let prepared = store.prepare_roster_source_enrollment_v1(
        inputs,
        RosterEnrollmentReviewV1 {
            operation: RosterAuthorityTextV1::parse("controlled-crawl-inspection")?,
            actor: RosterReviewActorV1::LocalOperator,
            reviewed_at: DateTime::parse_from_rfc3339("2026-10-16T00:49:43Z")?.to_utc(),
            rationale: RosterAuthorityTextV1::parse(
                "Controlled parser and grant integration test",
            )?,
        },
    )?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    store.apply_roster_source_enrollment_v1(prepared, &approved, &digest)?;
    Ok((directory, store))
}

pub(in crate::participation_evidence::roster_source_issuance) fn install_body(
    store: &Store,
    directory_body: &str,
    roster_body: &str,
) -> TestResult<(
    ProviderSchoolPacketV1,
    CanonicalSchool,
    CanonicalTeam,
    RosterIssuanceInspectionRequestV1,
)> {
    let packet = ProviderSchoolPacketV1::new(
        retained("https://mi.milesplit.com/teams", directory_body)?,
        retained(URL, roster_body)?,
    )?;
    let checked = packet.read()?;
    let school = checked.school().clone();
    let year = SchoolYear::new(2026).ok_or("year")?;
    let program = CanonicalTeam {
        id: CanonicalTeam::mint(&school.id, Sport::IndoorTrack, Gender::Boys, year),
        school: school.id.clone(),
        sport: Sport::IndoorTrack,
        gender: Gender::Boys,
        school_year: year,
        level: None,
        source_identities: vec![SourceIdentity::new(SourceNamespace::MilesplitTeam, "12439")],
        evidence: Vec::new(),
        retained_conflicts: Vec::new(),
    };
    store.append(Table::Schools, &school)?;
    store.append(Table::Teams, &program)?;
    store.append(
        Table::SourceObservations,
        &SourceObservation::School(checked.observation().clone()),
    )?;
    let schema = RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1;
    let reference = packet.reference()?;
    store.journal_done(
        schema.phase(),
        reference.as_str(),
        &serde_json::to_value(&packet)?,
    )?;
    let held = store
        .snapshot()
        .read_retained_capture_root_v1(
            schema,
            reference.as_str(),
            &mut AttachmentReplayGrantV1::new(),
        )?
        .ok_or("root missing")?;
    let request = RosterIssuanceInspectionRequestV1 {
        root: held.root().clone(),
        school: school.id.clone(),
        program: program.id.clone(),
    };
    Ok((packet, school, program, request))
}

pub(in crate::participation_evidence::roster_source_issuance) fn install(
    store: &Store,
) -> TestResult<(
    ProviderSchoolPacketV1,
    CanonicalSchool,
    CanonicalTeam,
    RosterIssuanceInspectionRequestV1,
)> {
    install_body(store, SMALL_DIRECTORY, SMALL_ROSTER)
}

pub(super) fn object(bytes: &[u8]) -> Value {
    json!({"digest":hash(bytes), "bytes":bytes.len()})
}

pub(in crate::participation_evidence::roster_source_issuance) fn actual_proposal_v3(
    packet: &ProviderSchoolPacketV1,
    school: &CanonicalSchool,
    program: &CanonicalTeam,
    evidence: &RosterSourceEvidenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult<LocallyCheckedRosterIssuanceV1> {
    actual_proposal_version(packet, school, program, evidence, grant, true)
}

fn actual_proposal_version(
    packet: &ProviderSchoolPacketV1,
    school: &CanonicalSchool,
    program: &CanonicalTeam,
    evidence: &RosterSourceEvidenceV1,
    grant: &mut AttachmentReplayGrantV1,
    row_local: bool,
) -> TestResult<LocallyCheckedRosterIssuanceV1> {
    let roster_body = packet.roster().body()?;
    let mut value = proposal(packet, &roster_body)?;
    let compiled = compiled_roster_parser_manifest()?;
    let checked = crate::participation_evidence::roster_source_issuance::manifest::assess_retained_roster_parser_manifest(
        compiled.canonical_bytes(), &compiled.parser_claim()?, grant)?;
    let objects = if row_local {
        crate::participation_evidence::roster_source_issuance::prepare_retained_roster_capture_objects_v3(
            evidence, &checked, grant)?
    } else {
        crate::participation_evidence::roster_source_issuance::capture_objects::prepare_retained_roster_capture_objects(
            evidence, &checked, grant)?
    };
    let contracts = compiled_roster_interpretation_contracts_v1();
    for (path, replacement) in [
        (
            "/scope/policy",
            serde_json::to_value(contracts.scope_reference()?)?,
        ),
        (
            "/binding/year_convention",
            serde_json::to_value(contracts.year_reference()?)?,
        ),
        (
            "/binding/projection_merge_contract",
            serde_json::to_value(contracts.projection_merge_reference()?)?,
        ),
        ("/capture/manifest", object(objects.metadata_bytes(grant)?)),
        (
            "/capture/interpretation",
            object(objects.interpretation_bytes(grant)?),
        ),
        ("/capture/occurrence", json!(objects.occurrence(grant)?)),
        ("/capture/acquired_at", json!(objects.acquired_at(grant)?)),
        ("/parser", serde_json::to_value(compiled.parser_claim()?)?),
        ("/scope/sport", json!("indoor_track")),
        ("/binding/sport", json!("indoor_track")),
        ("/binding/school", json!(school.id)),
        ("/binding/program", json!(program.id)),
        ("/binding/store", json!(evidence.store_identity())),
        (
            "/binding/snapshot_revision",
            json!(evidence.snapshot_sequence()),
        ),
        (
            "/capture/retained_source_admission",
            object(&evidence.retained_source_admission_manifest(grant)?),
        ),
        (
            "/capture/verification_grant",
            object(&evidence.verification_grant_manifest(grant)?),
        ),
        (
            "/binding/school_projection",
            object(&serde_json::to_vec(school)?),
        ),
        (
            "/binding/program_projection",
            object(&serde_json::to_vec(program)?),
        ),
        (
            "/binding/physical_projection_inventory",
            object(&evidence.physical_projection_manifest(grant)?),
        ),
    ] {
        *value.pointer_mut(path).ok_or("proposal field missing")? = replacement;
    }
    decode(&value)
}
