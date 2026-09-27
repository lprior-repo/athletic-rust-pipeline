use super::*;
use clap::Parser;

#[derive(clap::Parser)]
#[command(
    about = "A parser wrapper, because `Target` is a flattened argument group rather than a command"
)]
struct Wrapper {
    #[command(flatten)]
    target: Target,
}

#[test]
fn a_flag_free_invocation_uses_the_project_node_and_a_store_flag_goes_offline() {
    let flag_free = Wrapper::try_parse_from(["xtask"]).expect("flag-free invocation parses");
    match flag_free.target.mode().expect("mode") {
        Mode::Ingress(origin) => assert_eq!(origin, ingress::NODE_ORIGIN),
        Mode::Offline(path) => panic!("expected the ingress path, got offline {path:?}"),
    }

    let offline =
        Wrapper::try_parse_from(["xtask", "--store", "/tmp/store"]).expect("store parses");
    match offline.target.mode().expect("mode") {
        Mode::Offline(path) => assert_eq!(path, std::path::Path::new("/tmp/store")),
        Mode::Ingress(origin) => panic!("expected the offline path, got ingress {origin}"),
    }

    let both = Wrapper::try_parse_from(["xtask", "--store", "/tmp/store", "--ingress"]);
    assert!(both.is_err(), "both flags must be refused by the parser");
}
