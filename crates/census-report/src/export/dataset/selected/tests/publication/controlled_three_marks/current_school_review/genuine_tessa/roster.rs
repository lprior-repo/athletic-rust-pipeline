use super::*;
use census_crawl::milesplit::{RosterRejectionKind, RosterVerdict};
use census_domain::roster_affiliation_authority::RosterAuthorityTextV1;
use census_store::{
    RosterEnrollmentReviewV1, RosterIssuanceInspectionRequestV1, RosterReviewActorV1,
    RosterSourceReviewDecisionV1, RosterSourceReviewFieldsV1,
};

mod manifest;

pub(super) fn accept(installed: &source::Installed) -> TestResult<census_store::Receipt> {
    let packet = source::school_packet()?;
    source_facts(installed, &packet)?;
    enroll(&installed.store)?;
    let snapshot = installed.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let retained = snapshot
        .read_retained_capture_root_v1(
            RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1,
            packet.reference()?.as_str(),
            &mut grant,
        )?
        .ok_or("actual retained Roncalli school packet")?;
    let request = RosterIssuanceInspectionRequestV1 {
        root: retained.root().clone(),
        school: installed.school.id.clone(),
        program: installed.program.id.clone(),
    };
    let mut inspection = installed
        .store
        .inspect_roster_source_issuance_v1(request, grant)?;
    let fields = fields()?;
    let (evidence, grant) = inspection.source_parts()?;
    let (issuance, supplied) = controlled::current_school_review::proposal::prepare(
        &installed.school,
        &installed.program,
        evidence,
        &fields,
        grant,
    )?;
    manifest::original(
        evidence,
        issuance.binding().physical_projection_inventory(),
        grant,
    )?;
    let prepared = installed.store.prepare_roster_source_review_v1(
        inspection,
        issuance.into_proposal(),
        fields,
        supplied,
    )?;
    let bytes = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    match installed
        .store
        .apply_roster_source_review_v1(prepared, &bytes, &digest)?
    {
        Application::Written(receipt) => Ok(receipt),
        Application::Repeated(_) => {
            Err("genuine Roncalli roster review unexpectedly repeated".into())
        }
    }
}

fn source_facts(installed: &source::Installed, packet: &ProviderSchoolPacketV1) -> TestResult {
    let checked = packet.read()?;
    check!(eq; checked.school(), &installed.school);
    check!(eq; checked.reference(), &installed.school_reference);
    let RosterVerdict::Partial {
        roster,
        rejected,
        unfinished,
    } = checked.roster_verdict()
    else {
        return Err("genuine Roncalli partial roster was relabeled".into());
    };
    check!(eq; roster.athletes.len(), 195);
    check!(eq; rejected.len(), 6);
    check!(unfinished.is_none());
    check!(rejected
        .iter()
        .all(|row| row.kind == RosterRejectionKind::InvalidGraduationYear));
    check!(eq;
        rejected.iter().map(|row| row.athlete_id.as_deref()).collect::<Vec<_>>(),
        vec![Some("14473113"), Some("14112371"), Some("14292663"), Some("14292654"), Some("14292664"), Some("14287285")]
    );
    let mut rows = roster
        .athletes
        .iter()
        .filter(|row| row.athlete().athlete_id == "13658157");
    let row = rows
        .next()
        .ok_or("genuine Tessa native current roster row")?;
    check!(rows.next().is_none());
    check!(eq; row.athlete().name, "Tessa Mize");
    check!(eq; row.athlete().gender, Gender::Girls);
    check!(eq; row.athlete().grad_year, GradYear::CO2027);
    let current = row
        .current_roster()
        .ok_or("literal scoped Current Roster Tessa statement")?;
    check!(current.supports(Sport::IndoorTrack));
    current.source_statements_v1()?;
    check!(eq; packet.roster().acquired_at().date_naive().to_string(), "2026-10-21");
    check!(eq; installed.program.school, installed.school.id);
    check!(eq; installed.program.sport, Sport::IndoorTrack);
    check!(eq; installed.program.gender, Gender::Girls);
    check!(eq; installed.program.school_year.get(), 2026);
    Ok(())
}

fn enroll(store: &Store) -> TestResult {
    let inputs = store.inspect_roster_source_enrollment_v1()?;
    let prepared = store.prepare_roster_source_enrollment_v1(inputs, RosterEnrollmentReviewV1 {
        operation: RosterAuthorityTextV1::parse("genuine-roncalli-roster-report-qualification-enroll")?,
        actor: RosterReviewActorV1::LocalOperator, reviewed_at: at()?,
        rationale: RosterAuthorityTextV1::parse("Controlled qualification review of full genuine Roncalli original bytes, preserving the Partial roster verdict and all rejection diagnostics; no owned production acquisition or authenticated human approval claim")?,
    })?;
    let bytes = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    check!(matches!(
        store.apply_roster_source_enrollment_v1(prepared, &bytes, &digest)?,
        Application::Written(_)
    ));
    Ok(())
}

fn fields() -> TestResult<RosterSourceReviewFieldsV1> {
    Ok(RosterSourceReviewFieldsV1 {
        operation: RosterAuthorityTextV1::parse("genuine-roncalli-201-row-current-roster-report-qualification")?,
        actor: RosterReviewActorV1::LocalOperator,
        decision: RosterSourceReviewDecisionV1::QualifyAndActivateExactCaptureRepresentation,
        reviewed_at: at()?,
        rationale: RosterAuthorityTextV1::parse("Test reviewer qualifies the complete connected 201-row source inventory: 195 admitted rows, six invalid graduation-year rejections, and no unfinished locator. Literal Current Roster Tessa13658157 female2027 indoor is capture-day scoped; no tenure interval inferred")?,
    })
}

fn at() -> TestResult<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339("2026-10-21T22:00:00Z")?.to_utc())
}

pub(super) fn require_manifest(store: &Store) -> TestResult {
    manifest::current(store)
}
