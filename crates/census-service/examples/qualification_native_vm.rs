#![forbid(unsafe_code)]

#[path = "qualification_native_vm/mod.rs"]
mod qualification_native_vm;

fn main() -> anyhow::Result<()> {
    qualification_native_vm::run()
}
