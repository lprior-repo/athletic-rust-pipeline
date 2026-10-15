#![cfg(any(
    not(unix),
    target_os = "cygwin",
    target_os = "emscripten",
    target_os = "espidf",
    target_os = "fuchsia",
    target_os = "horizon",
    target_os = "openbsd",
    target_os = "redox",
    target_os = "vita",
    target_os = "wasi"
))]

#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use census_crawl::convert::{
    converter_files_capped, converter_stdout_capped, ConverterLifecycleError,
};
use census_crawl::CrawlError;
use std::path::Path;
use std::time::Duration;

#[test]
fn converter_unsupported_process_platform_refuses_before_spawn(
) -> Result<(), Box<dyn std::error::Error>> {
    let command = "/converter-must-not-spawn";
    let path = Path::new("/converter-must-not-open");
    for outcome in [
        converter_stdout_capped(command, ["unused"], b"", Duration::from_secs(1), 1).map(drop),
        converter_files_capped(command, ["unused"], path, Duration::from_secs(1), 1),
    ] {
        let Err(CrawlError::ConverterLifecycle(ConverterLifecycleError::UnsupportedPlatform {
            command: held,
        })) = outcome
        else {
            return Err(format!("expected typed unsupported process platform: {outcome:?}").into());
        };
        check!(eq; held, command);
    }
    Ok(())
}
