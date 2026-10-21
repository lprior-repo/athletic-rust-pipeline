use census_domain::model::{
    DiscoveryPriorityV1, Gender, GradYear, PerformanceWindowV1, ProfileDate, ProfileDateRangeV1,
    ProfilePoliciesV1, ProvisionalInclusionV1, QueryProfileInputV1, QueryProfileV1,
    RegionalMembershipV1, SchoolYear, Sport,
};
use census_domain::profile_execution::{
    selected_qualification_contract_ref_v5 as reference, ContractRoleV1 as Role,
};
use census_domain::UsJurisdiction;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args_os()
        .nth(1)
        .is_some_and(|arg| arg == "--validate-captures")
    {
        let root = std::env::args_os().nth(2).ok_or("capture directory")?;
        return validate_captures(std::path::Path::new(&root));
    }
    if let Some(root) = std::env::args_os().nth(1) {
        if let Some(selection) = std::env::args_os().nth(2) {
            return inspect_selected(
                std::path::Path::new(&root),
                std::path::Path::new(&selection),
            );
        }
        return inspect_retained(std::path::Path::new(&root));
    }
    let profile = QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![UsJurisdiction::Michigan],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::OutdoorTrack],
        competition_categories: vec![Gender::Boys],
        as_of: ProfileDate::parse("2026-10-16")?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::new(2026).ok_or("school year")?,
        source_plan: reference(Role::SourcePlan)?,
        acquisition_window: ProfileDateRangeV1::new(
            ProfileDate::parse("2026-01-01")?,
            ProfileDate::parse("2026-10-16")?,
        )?,
        completion_contract: reference(Role::Completion)?,
        policies: ProfilePoliciesV1 {
            cohort: reference(Role::Cohort)?,
            event: reference(Role::Event)?,
            identity: reference(Role::Identity)?,
            affiliation: reference(Role::Affiliation)?,
            contact: reference(Role::Contact)?,
            participation: reference(Role::Participation)?,
            publication: reference(Role::Publication)?,
        },
        include_provisional: ProvisionalInclusionV1::Exclude,
    })?;
    serde_json::to_writer_pretty(std::io::stdout().lock(), profile.input())?;
    Ok(())
}

fn inspect_retained(root: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    use census_crawl::participation_evidence::roster_source_issuance::assess_reviewed_roster_source_historical_v2;
    use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
    let (store, _writer) = census_store::Store::open_current_qualification_v3(root)?.into_parts();
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
    for claim in owner.claims(&mut grant)? {
        let checked = assess_reviewed_roster_source_historical_v2(claim, &mut grant)?;
        let packet = checked.checked_packet(&mut grant)?;
        let members = packet
            .roster_verdict()
            .roster()
            .ok_or("reviewed source has no admitted roster rows")?
            .athletes
            .len();
        println!(
            "{}",
            serde_json::json!({
                "operation": claim.operation(),
                "school": packet.school().id,
                "source_roster_rows": members,
                "scope": "Existing retained-semantic assessor only; no approval or publication"
            })
        );
    }
    Ok(())
}

fn inspect_selected(
    root: &std::path::Path,
    selection_path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let selection: census_domain::model::ProfileSelectionV1 =
        serde_json::from_slice(&std::fs::read(selection_path)?)?;
    let (store, _writer) = census_store::Store::open_current_qualification_v3(root)?.into_parts();
    let snapshot = store.snapshot();
    let profile = snapshot.selected_profile(&selection)?;
    let objects = root
        .parent()
        .ok_or("qualification parent")?
        .join("genuine-co2027-selection-objects");
    std::fs::create_dir_all(&objects)?;
    let dataset = census_report::export::ExportDataset::from_selected_snapshot(
        &store, &snapshot, &selection, &objects,
    )?;
    let outcome = census_report::bests::build_with_exclusions(
        &dataset,
        &census_report::bests::Options {
            scope: census_report::report::Scope::AllSources,
            grad_year: Some(2027),
            limit: None,
            jurisdictions: profile.input().jurisdictions.clone(),
        },
    )?;
    println!(
        "{}",
        serde_json::json!({
            "store": selection.store_identity(),
            "profile": selection.profile_id(),
            "raw_athletes": dataset.athletes.len(),
            "raw_performances": dataset.performances.len(),
            "selected_best_marks": outcome.selections.len(),
            "participation_exclusions": outcome.participation_exclusions.len(),
            "combined_exclusions": outcome.combined_exclusions.len()
        })
    );
    if outcome.selections.is_empty() {
        return Err("genuine co2027 workbook blocked: no eligible selected best marks".into());
    }
    Ok(())
}

fn validate_captures(root: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    use census_crawl::participation_evidence::{
        provider_target::SourceTargetPacketV1, result_ownership::read_retained_html_result_rows,
        RetainedHtmlCaptureV1,
    };
    let capture = |name: &str| -> Result<RetainedHtmlCaptureV1, Box<dyn std::error::Error>> {
        Ok(serde_json::from_slice(&std::fs::read(root.join(name))?)?)
    };
    let target = SourceTargetPacketV1::new(
        capture("owen-profile.capture.json")?,
        capture("chsl-linked.capture.json")?,
    )?;
    let checked = target.read()?;
    let raw = capture("chsl-760382-1300204.capture.json")?;
    let locator = census_crawl::milesplit::SourceRowLocator {
        ordinal: 93,
        byte_offset: 46412,
        byte_length: 92,
    };
    let document = read_retained_html_result_rows(&raw, &[locator])?;
    let row = document.row(0).ok_or("original Owen row")?;
    let parsed = row.parsed_row();
    println!(
        "{}",
        serde_json::json!({
            "source_person": checked.profile().person().id,
            "profile_name": checked.profile().name(),
            "profile_cohort": checked.profile().cohort(),
            "competition_evidence": format!("{:?}", checked.categories()),
            "original_result_name": parsed.name,
            "original_result_school": parsed.school,
            "original_result_grade": parsed.grade.map(|grade| grade.get()),
            "original_result_mark": format!("{:?}", parsed.mark),
            "original_result_day": document.published_start(),
            "original_row_reference": row.reference(),
            "original_row": row.original_row()?,
            "scope": "Native genuine source intersection; no Store or accepted facts"
        })
    );
    let encoded = target.encode()?;
    let path = root.join("owen-target.packet.json");
    if path.exists() && std::fs::read(&path)? != encoded {
        return Err("refusing to replace original canonical target packet".into());
    }
    std::fs::write(path, encoded)?;
    Ok(())
}
