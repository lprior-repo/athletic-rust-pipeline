use crate::cmd::Cmd;
use anyhow::Result;

pub(crate) fn run(args: Vec<String>) -> Result<()> {
    Cmd::new("cargo")
        .arg("run")
        .arg("--release")
        .arg("-p")
        .arg("census-store")
        .arg("--example")
        .arg("keyspace_ab")
        .arg("--")
        .args(args)
        .run()
}
