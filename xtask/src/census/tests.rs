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
