use super::*;
use clap::Parser;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn fresh_national_request_selects_managed_contract_without_changing_requested_scope() -> TestResult
{
    let cli = Cli::try_parse_from([
        "census-service",
        "national",
        "--states",
        "WI,IA",
        "--revision",
        "3",
        "--pass-budget",
        "17",
        "--pass-delay-seconds",
        "41",
    ])?;
    let crate::cli::command::Command::National(args) = &cli.command else {
        return Err("fixture parsed a different command".into());
    };
    let season = SchoolYear::new(2026).ok_or("fixture season")?;
    let jurisdictions = [UsJurisdiction::Wisconsin, UsJurisdiction::Iowa];
    let history = HistoryWindow::cohort("2026-10-12")?;
    let request = fresh_request(&cli, args, season, Revision(3), &jurisdictions, history)?;
    check!(eq; request.publication_contract, PublicationContract::ManagedBundleV2);
    check!(eq; request.season, season);
    check!(eq; request.revision, Revision(3));
    check!(eq; request.jurisdictions, jurisdictions);
    check!(eq; request.history, history);
    check!(eq; request.pass_budget, 17);
    check!(eq; request.pass_delay_seconds, 41);
    Ok(())
}

#[test]
fn selected_national_request_requires_and_preserves_both_selection_ids() -> TestResult {
    let cli = Cli::try_parse_from([
        "census-service",
        "national",
        "--selected-store-identity",
        &"a".repeat(64),
        "--selected-profile-id",
        &"b".repeat(64),
    ])?;
    let crate::cli::command::Command::National(args) = &cli.command else {
        return Err("fixture parsed a different command".into());
    };
    let season = SchoolYear::new(2026).ok_or("fixture season")?;
    let history = HistoryWindow::cohort("2026-10-12")?;
    let request = fresh_request(&cli, args, season, Revision(1), &[], history)?;
    check!(eq; request.publication_contract, PublicationContract::SelectedPublicationV1);
    check!(matches!(
        request.export_job,
        ExportJobRequest::SelectedPublicationV1 { .. }
    ));

    let missing = Cli::try_parse_from([
        "census-service",
        "national",
        "--selected-store-identity",
        &"a".repeat(64),
    ])?;
    let crate::cli::command::Command::National(args) = &missing.command else {
        return Err("fixture parsed a different command".into());
    };
    check!(fresh_request(&missing, args, season, Revision(1), &[], history).is_err());
    Ok(())
}
