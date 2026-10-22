#![forbid(unsafe_code)]

use std::process::ExitCode;

use census_service::bootstrap::{serve_command, ServeOptions};

fn main() -> ExitCode {
    let options = match ServeOptions::from_env(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(error @ census_service::bootstrap::BootstrapError::HelpRequested) => {
            println!("{error}");
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            eprintln!("census-serve: {error}");
            return ExitCode::FAILURE;
        }
    };
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("census-serve: runtime startup failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    runtime.block_on(async {
        match serve_command(options).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("census-serve: {error:#}");
                ExitCode::FAILURE
            }
        }
    })
}
