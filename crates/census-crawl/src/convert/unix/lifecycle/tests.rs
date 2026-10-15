use super::*;
use rustix::process::{waitpid, WaitOptions};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn converter_foreign_reap_preserves_primary_and_each_unproven_cleanup_obligation() -> TestResult {
    let mut child = Command::new("/bin/true")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()?;
    let pid = Pid::from_child(&child);
    let Some((reaped, status)) = waitpid(Some(pid), WaitOptions::empty())? else {
        return Err("fixture leader did not become waitable".into());
    };
    check!(eq; reaped, pid);
    check!(eq; status.exit_status(), Some(0));
    let outcome: CrawlResult<()> = terminate(&mut child, "/bin/true").finish(
        "/bin/true",
        Err(CrawlError::Resource {
            resource: "converter output bytes",
            requested: 2,
            limit: 1,
        }),
    );
    let Err(CrawlError::ConverterLifecycle(ConverterLifecycleError::Multiple {
        primary,
        secondary,
    })) = outcome
    else {
        return Err(
            format!("primary and cleanup errors were not both retained: {outcome:?}").into(),
        );
    };
    check!(matches!(
        *primary,
        CrawlError::Resource {
            resource: "converter output bytes",
            requested: 2,
            limit: 1,
        }
    ));
    let CrawlError::ConverterLifecycle(ConverterLifecycleError::Multiple { primary, secondary }) =
        *secondary
    else {
        return Err("identity and wait errors must both be retained".into());
    };
    check_unproven(*primary, ConverterObligation::LeaderReaped)?;
    check_unproven(*secondary, ConverterObligation::LeaderIdentity)
}

fn check_unproven(error: CrawlError, expected: ConverterObligation) -> TestResult {
    let CrawlError::ConverterLifecycle(ConverterLifecycleError::Unproven {
        command,
        obligation,
        source,
    }) = error
    else {
        return Err(format!("expected typed cleanup failure: {error:?}").into());
    };
    check!(eq; command, "/bin/true");
    check!(eq; obligation, expected);
    check!(eq; source.raw_os_error(), Some(Errno::CHILD.raw_os_error()));
    Ok(())
}
