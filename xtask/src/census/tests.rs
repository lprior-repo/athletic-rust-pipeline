use super::*;
use clap::Parser;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

#[derive(clap::Parser)]
#[command(
    about = "A parser wrapper, because `Target` is a flattened argument group rather than a command"
)]
struct Wrapper {
    #[command(flatten)]
    target: Target,
}

#[test]
fn a_flag_free_invocation_uses_the_project_node_and_a_store_flag_goes_offline() -> TestResult {
    let flag_free = Wrapper::try_parse_from(["xtask"])?;
    match flag_free.target.mode()? {
        Mode::Ingress(origin) => check!(eq; origin, ingress::NODE_ORIGIN),
        Mode::Offline(path) => {
            return Err(anyhow::anyhow!("expected the ingress path, got offline {path:?}").into())
        }
    }

    let offline = Wrapper::try_parse_from(["xtask", "--store", "/tmp/store"])?;
    match offline.target.mode()? {
        Mode::Offline(path) => check!(eq; path, std::path::Path::new("/tmp/store")),
        Mode::Ingress(origin) => {
            return Err(anyhow::anyhow!("expected the offline path, got ingress {origin}").into())
        }
    }

    let both = Wrapper::try_parse_from(["xtask", "--store", "/tmp/store", "--ingress"]);
    check!(both.is_err(), "both flags must be refused by the parser");
    Ok(())
}

fn export_options(generation: Option<&str>) -> ExportRequest {
    ExportRequest {
        target: Target {
            store: None,
            ingress: Some(ingress::NODE_ORIGIN.to_string()),
        },
        out: None,
        grad_year: 2027,
        school_year: 2026,
        core: true,
        limit: Some(50),
        generation: generation.map(str::to_string),
        selected_store_identity: Some("a".repeat(64)),
        selected_profile_id: Some("b".repeat(64)),
    }
}

#[test]
fn both_entries_key_one_workbook_generation() -> TestResult {
    use census_service::restate_services::{workbook_request_key, ExportGeneration};

    let options = export_options(Some("2"));
    let request = service::workbook_request(&options)?;
    let generation = ExportGeneration::resolve(options.generation.as_deref())?;
    let key = workbook_request_key(&request, &generation)?;
    let segments: Vec<&str> = key.split(':').collect();
    check!(eq; segments.len(), 9);
    check!(
        key.starts_with("workbook:2027:core:50:all:.:2026:selected_v1_"),
        "the xtask entry keys the selected request exactly like the CLI-live entry"
    );
    check!(eq; segments[8], generation.as_str());

    let repeated = service::workbook_request(&export_options(Some("2")))?;
    check!(eq;
        workbook_request_key(&repeated, &generation)?,
        key,
        "the same explicit generation reattaches the same workflow"
    );
    let other_generation = ExportGeneration::resolve(Some("3"))?;
    check!(ne; workbook_request_key(&request, &other_generation)?, key);

    let omitted = export_options(None);
    check!(
        ExportGeneration::resolve(omitted.generation.as_deref()).is_err(),
        "an omitted generation is refused instead of reusing legacy generation 1 keys"
    );
    Ok(())
}
