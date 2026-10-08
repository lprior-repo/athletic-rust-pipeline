#![recursion_limit = "256"]

#[path = "qualification_bound_projection/mod.rs"]
mod qualification_bound_projection;

fn main() -> qualification_bound_projection::Result<()> {
    qualification_bound_projection::run()
}
