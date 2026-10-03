#![forbid(unsafe_code)]

#[path = "qualification_postal/mod.rs"]
mod qualification_postal;

fn main() -> anyhow::Result<()> {
    qualification_postal::execute()
}
