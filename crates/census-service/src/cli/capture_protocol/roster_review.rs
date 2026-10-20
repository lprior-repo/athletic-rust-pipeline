use anyhow::{ensure, Context, Result};
use census_crawl::participation_evidence::provider_school::ProviderSchoolPacketRefV1;
use census_domain::model::{tag, IdTag, SchoolId, TeamId};
use census_domain::roster_affiliation_authority::RosterAuthorityTextV1;
use census_store::{
    RosterIssuanceInspectionRequestV1, RosterReviewActorV1, RosterSourceReviewDecisionV1,
    RosterSourceReviewFieldsV1, Store,
};
use clap::Args;
use std::io::Write;
use std::path::Path;

pub(super) mod approval;
pub(super) mod proposal;

#[derive(Debug, Args)]
pub(super) struct RosterReviewArgs {
    #[arg(
        long,
        value_name = "PACKET_REF",
        help = "Retained ORIGINAL provider-school packet key: 64 lowercase hex characters from the intake reply"
    )]
    packet_ref: String,
    #[arg(
        long,
        value_name = "SCHOOL_ID",
        help = "School under review, sch_<hex>; must match the retained school row and the derived proposal"
    )]
    school: String,
    #[arg(
        long,
        value_name = "TEAM_ID",
        help = "Program under review, team_<hex>; must match the retained program row and the derived proposal"
    )]
    program: String,
    #[arg(
        long,
        value_name = "OPERATION",
        help = "Stable explicit review operation label; part of the approval hash, never acceptance by itself"
    )]
    operation: String,
    #[arg(
        long,
        value_name = "RFC3339",
        help = "Actual review instant in RFC3339; commits before it are refused"
    )]
    reviewed_at: String,
    #[arg(
        long,
        help = "Operator rationale in own words; bounded review text, never an identity assertion"
    )]
    rationale: String,
    #[arg(
        long,
        value_name = "SHA256",
        help = "SHA256 of the previewed opaque approval bytes; absent means preview only and nothing is applied"
    )]
    approve_sha256: Option<String>,
}

pub(super) fn run_to<W: Write>(root: &Path, args: &RosterReviewArgs, output: &mut W) -> Result<()> {
    let (reference, school, program, operation, rationale, reviewed_at) = parse(args)?;
    let (store, _writer) = Store::open_current_qualification_v3(root)
        .context("opening the stopped current-qualification store")?
        .into_parts();
    let held = proposal::retained(&store, &reference)?;
    let packet = proposal::packet(&held)?;
    let request = RosterIssuanceInspectionRequestV1 {
        root: held.root().clone(),
        school,
        program,
    };
    let mut inspection = store
        .inspect_roster_source_issuance_v1(
            request,
            census_domain::evidence_subject_attachment::AttachmentReplayGrantV1::new(),
        )
        .context("inspecting the roster source issuance")?;
    let (proposal, supplied) = proposal::derive(&packet, &mut inspection)
        .context("deriving the source-backed proposal")?;
    let fields = RosterSourceReviewFieldsV1 {
        operation,
        actor: RosterReviewActorV1::LocalOperator,
        decision: RosterSourceReviewDecisionV1::QualifyAndActivateExactCaptureRepresentation,
        reviewed_at,
        rationale,
    };
    let packet_reference = reference.as_str().to_owned();
    let requested_operation = args.operation.clone();
    let mut prepared = store
        .prepare_roster_source_review_v1(inspection, proposal, fields, supplied)
        .context("preparing the roster source review")?;
    let scoped = approval::verify(&mut prepared)?;
    approval::conclude(
        &store,
        prepared,
        args.approve_sha256.as_deref(),
        &packet_reference,
        &requested_operation,
        scoped,
        output,
    )
}

fn parse(
    args: &RosterReviewArgs,
) -> Result<(
    ProviderSchoolPacketRefV1,
    SchoolId,
    TeamId,
    RosterAuthorityTextV1,
    RosterAuthorityTextV1,
    chrono::DateTime<chrono::Utc>,
)> {
    let reference = ProviderSchoolPacketRefV1::parse(&args.packet_ref)
        .context("parsing the retained packet reference")?;
    let school: SchoolId = serde_json::from_value(serde_json::Value::String(args.school.clone()))
        .context("parsing the school id")?;
    ensure!(
        school.validate(tag::School::PREFIX),
        "school id prefix differs"
    );
    let program: TeamId = serde_json::from_value(serde_json::Value::String(args.program.clone()))
        .context("parsing the program id")?;
    ensure!(
        program.validate(tag::Team::PREFIX),
        "program id prefix differs"
    );
    let operation = RosterAuthorityTextV1::parse(&args.operation)
        .context("parsing the stable explicit review operation")?;
    let rationale =
        RosterAuthorityTextV1::parse(&args.rationale).context("parsing the operator rationale")?;
    let reviewed_at = chrono::DateTime::parse_from_rfc3339(&args.reviewed_at)
        .context("parsing the review instant as RFC3339")?
        .to_utc();
    Ok((
        reference,
        school,
        program,
        operation,
        rationale,
        reviewed_at,
    ))
}
