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
    }
}

#[test]
fn both_entries_key_one_workbook_generation() -> TestResult {
    use census_service::restate_services::{workbook_request_key, ExportGeneration};

    let options = export_options(Some("2"));
    let request = service::workbook_request(&options)?;
    let generation = ExportGeneration::resolve(options.generation.as_deref())?;
    let key = workbook_request_key(&request, &generation)?;
    check!(eq;
        key,
        "workbook:2027:core:50:.:2026:2",
        "the xtask entry keys the selected generation exactly like the CLI-live entry"
    );

    let legacy_options = export_options(None);
    let legacy_request = service::workbook_request(&legacy_options)?;
    let legacy_generation = ExportGeneration::resolve(legacy_options.generation.as_deref())?;
    let legacy_key = workbook_request_key(&legacy_request, &legacy_generation)?;
    check!(eq;
        legacy_key,
        "workbook:2027:core:50:.:2026:1",
        "an omitted flag reuses the legacy generation 1 keys on every entry"
    );
    check!(ne;
        key, legacy_key,
        "a selected generation must not reattach the legacy workflow"
    );
    Ok(())
}
