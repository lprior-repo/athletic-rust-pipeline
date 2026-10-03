#![forbid(unsafe_code)]

#[path = "qualification_native_teams/mod.rs"]
mod qualification_native_teams;

fn main() -> std::process::ExitCode {
    match qualification_native_teams::execute() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("native teams qualification: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}
