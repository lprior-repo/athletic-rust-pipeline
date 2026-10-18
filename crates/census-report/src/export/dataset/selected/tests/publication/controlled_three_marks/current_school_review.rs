use super::source::{CurrentSchoolInput, InstalledSource, TestResult};
use census_crawl::milesplit::{roster_entities, RosterVerdict, Site};
use census_crawl::participation_evidence::provider_school::{
    CheckedProviderSchoolPacketV1, ProviderSchoolPacketV1,
};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{CanonicalSchool, CanonicalTeam, SchoolYear};
use census_domain::roster_affiliation_authority::RosterAuthorityTextV1;
use census_store::{
    Application, Receipt, RetainedCaptureRootSchemaV1, RosterEnrollmentReviewV1,
    RosterIssuanceInspectionRequestV1, RosterReviewActorV1, RosterSourceReviewDecisionV1,
    RosterSourceReviewFieldsV1, Store, Table,
};
use chrono::{DateTime, Utc};

mod proposal;

pub(super) fn accept(
    installed: &InstalledSource,
    input: &CurrentSchoolInput,
) -> TestResult<Receipt> {
    enroll(&installed.store)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let (request, school, program) = prerequisite(&installed.store, input, &mut grant)?;
    let mut inspection = installed
        .store
        .inspect_roster_source_issuance_v1(request, grant)?;
    let fields = review_fields()?;
    let (evidence, grant) = inspection.source_parts()?;
    let (issuance, supplied) = proposal::prepare(&school, &program, evidence, &fields, grant)?;
    let prepared = installed
        .store
        .prepare_roster_source_review_v1(inspection, issuance, fields, supplied)?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    match installed
        .store
        .apply_roster_source_review_v1(prepared, &approved, &digest)?
    {
        Application::Written(receipt) => Ok(receipt),
        Application::Repeated(_) => {
            Err("controlled current school review unexpectedly replayed".into())
        }
    }
}

fn enroll(store: &Store) -> TestResult {
    let inputs = store.inspect_roster_source_enrollment_v1()?;
    let prepared = store.prepare_roster_source_enrollment_v1(
        inputs,
        RosterEnrollmentReviewV1 {
            operation: RosterAuthorityTextV1::parse("controlled-current-school-roster-enrollment")?,
            actor: RosterReviewActorV1::LocalOperator,
            reviewed_at: review_time()?,
            rationale: RosterAuthorityTextV1::parse(
                "Controlled roster source enrollment; not genuine-source qualification",
            )?,
        },
    )?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    match store.apply_roster_source_enrollment_v1(prepared, &approved, &digest)? {
        Application::Written(_) => Ok(()),
        Application::Repeated(_) => {
            Err("controlled roster enrollment unexpectedly replayed".into())
        }
    }
}

fn prerequisite(
    store: &Store,
    input: &CurrentSchoolInput,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult<(
    RosterIssuanceInspectionRequestV1,
    CanonicalSchool,
    CanonicalTeam,
)> {
    let view = store.snapshot();
    let retained = view
        .read_retained_capture_root_v1(
            RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1,
            input.reference.as_str(),
            grant,
        )?
        .ok_or("retained provider school root missing")?;
    let packet: ProviderSchoolPacketV1 =
        serde_json::from_slice(retained.payload().get().as_bytes())?;
    let checked = packet.read()?;
    if checked.reference() != &input.reference || checked.school().id != input.school {
        return Err("current school input differs from retained packet".into());
    }
    let school = view
        .get_merged::<CanonicalSchool>(Table::Schools, input.school.as_str())?
        .ok_or("current school intake projection missing")?;
    let projected = project_program(&packet, &checked)?;
    let program = view
        .get_merged::<CanonicalTeam>(Table::Teams, projected.id.as_str())?
        .ok_or("source-derived current program missing")?;
    let request = RosterIssuanceInspectionRequestV1 {
        root: retained.root().clone(),
        school: input.school.clone(),
        program: program.id.clone(),
    };
    Ok((request, school, program))
}

fn project_program(
    packet: &ProviderSchoolPacketV1,
    checked: &CheckedProviderSchoolPacketV1,
) -> TestResult<CanonicalTeam> {
    let RosterVerdict::Complete { roster } = checked.roster_verdict() else {
        return Err("controlled current roster must be complete".into());
    };
    let year = SchoolYear::from_date(&packet.roster().acquired_at().date_naive().to_string())
        .ok_or("current roster school year unsupported")?;
    let state = checked
        .school()
        .state
        .ok_or("current roster school jurisdiction missing")?;
    let (_, _, programs) = roster_entities(
        &roster.team,
        &roster.athletes,
        year,
        &packet.roster().acquired_at().to_rfc3339(),
        &Site::for_jurisdiction(state),
    )?;
    let [program]: [CanonicalTeam; 1] = programs
        .try_into()
        .map_err(|_| "controlled current roster must identify exactly one program")?;
    if program.school != checked.school().id {
        return Err("source-derived program belongs to a different school".into());
    }
    Ok(program)
}

fn review_fields() -> TestResult<RosterSourceReviewFieldsV1> {
    Ok(RosterSourceReviewFieldsV1 {
        operation: RosterAuthorityTextV1::parse("controlled-current-school-reviewed-roster")?,
        actor: RosterReviewActorV1::LocalOperator,
        decision: RosterSourceReviewDecisionV1::QualifyAndActivateExactCaptureRepresentation,
        reviewed_at: review_time()?,
        rationale: RosterAuthorityTextV1::parse(
            "Synthetic controlled current school review through public operations; no genuine-source or authenticated human approval claim",
        )?,
    })
}

fn review_time() -> TestResult<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339("2026-10-17T01:00:00Z")?.with_timezone(&Utc))
}
