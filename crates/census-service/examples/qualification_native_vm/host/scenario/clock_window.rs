use anyhow::{ensure, Context, Result};
use serde_json::Value;
use std::path::Path;

use super::super::super::{artifacts, transport::Ssh, GUEST};

use super::clock_field;

mod observation;

pub(super) use observation::ClockProgress;

pub(super) fn crossed_day(before: chrono::NaiveDate, after: chrono::NaiveDate) -> Result<bool> {
    ensure!(
        after == before || before.succ_opt() == Some(after),
        "clock oracle skipped or reversed an actual calendar day"
    );
    Ok(after != before)
}

pub(super) fn inject(ssh: &Ssh, root: &Path) -> Result<Value> {
    match ssh.action("clock-set") {
        Ok(evidence) => Ok(evidence),
        Err(error) => {
            let remote = format!("/usr/bin/cat {GUEST}/guest-clock-injection-validation.json");
            let retained = ssh
                .exec("clock-injection-rejected-evidence", &remote)
                .and_then(|raw| {
                    let evidence: Value = serde_json::from_str(&raw)?;
                    artifacts::publish(
                        &root.join("guest-clock-injection-validation.json"),
                        &evidence,
                    )
                });
            Err(error.context(format!(
                "guest clock injection refused; diagnostic retention: {retained:?}"
            )))
        }
    }
}

pub(super) fn validate_machine(before: &Value, after: &Value) -> Result<()> {
    let before_boot = clock_field(before, "boot_id")?;
    let after_boot = clock_field(after, "boot_id")?;
    let before_machine = clock_field(before, "machine_id")?;
    let after_machine = clock_field(after, "machine_id")?;
    ensure!(!before_boot.is_empty(), "boot_id empty in before");
    ensure!(!after_boot.is_empty(), "boot_id empty in after");
    ensure!(!before_machine.is_empty(), "machine_id empty in before");
    ensure!(!after_machine.is_empty(), "machine_id empty in after");
    ensure!(
        before_boot == after_boot,
        "boot_id changed between before and after"
    );
    ensure!(
        before_machine == after_machine,
        "machine_id changed between before and after"
    );
    Ok(())
}

pub(super) fn validate_injection(injected: &Value) -> Result<()> {
    injection_realtime(injected).map(|_| ())
}

fn injection_realtime(injected: &Value) -> Result<chrono::DateTime<chrono::Utc>> {
    let before = injected.get("before").context("missing before")?;
    let after = injected.get("after").context("missing after")?;
    validate_machine(before, after)?;
    let utc = reported_realtime(after)?;
    let before_date = chrono::NaiveDate::parse_from_str(clock_field(before, "date")?, "%Y-%m-%d")?;
    ensure!(
        utc.date_naive() == before_date,
        "before and after dates differ"
    );
    validate_window(utc)?;
    Ok(utc)
}

fn reported_realtime(clock: &Value) -> Result<chrono::DateTime<chrono::Utc>> {
    let utc = chrono::DateTime::parse_from_rfc3339(clock_field(clock, "realtime")?)?
        .with_timezone(&chrono::Utc);
    let reported_date = chrono::NaiveDate::parse_from_str(clock_field(clock, "date")?, "%Y-%m-%d")?;
    ensure!(
        utc.date_naive() == reported_date,
        "clock realtime {utc} does not match reported date {reported_date}"
    );
    Ok(utc)
}

fn validate_window(utc: chrono::DateTime<chrono::Utc>) -> Result<()> {
    let after_date = utc.date_naive();
    let target = after_date
        .and_hms_opt(23, 55, 0)
        .context("failed to construct target time")?;
    let midnight = after_date
        .succ_opt()
        .and_then(|day| day.and_hms_opt(0, 0, 0))
        .context("failed to construct midnight")?;
    ensure!(
        target <= utc.naive_utc() && utc.naive_utc() < midnight,
        "clock realtime {utc} outside pre-midnight window [{target}, {midnight}) UTC"
    );
    Ok(())
}

pub(super) fn validate_baseline(injected: &Value, clock: &Value) -> Result<()> {
    let injected_utc = injection_realtime(injected)?;
    let after = injected.get("after").context("missing after")?;
    validate_machine(after, clock)?;
    let baseline_utc = reported_realtime(clock)?;
    ensure!(
        baseline_utc.date_naive() == injected_utc.date_naive(),
        "baseline realtime {baseline_utc} differs from original injected date {}",
        injected_utc.date_naive()
    );
    validate_window(baseline_utc)?;
    ensure!(
        baseline_utc >= injected_utc,
        "baseline realtime {baseline_utc} precedes observed injection realtime {injected_utc}"
    );
    Ok(())
}

#[cfg(test)]
mod baseline_tests;

#[cfg(test)]
mod progress_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn failed_injection_at_root17_time_is_rejected() -> anyhow::Result<()> {
        let v = json!({
            "before": {
                "date": "2026-10-02",
                "realtime": "2026-10-02T19:56:54.892691734+00:00",
                "boot_id": "boot",
                "machine_id": "machine"
            },
            "after": {
                "date": "2026-10-02",
                "realtime": "2026-10-02T19:56:55.037242158+00:00",
                "boot_id": "boot",
                "machine_id": "machine"
            }
        });
        assert!(validate_injection(&v).is_err());
        Ok(())
    }

    #[test]
    fn target_injection_at_23_55_is_accepted() -> anyhow::Result<()> {
        let v = json!({
            "before": {
                "date": "2026-10-02",
                "realtime": "2026-10-02T23:55:00+00:00",
                "boot_id": "boot",
                "machine_id": "machine"
            },
            "after": {
                "date": "2026-10-02",
                "realtime": "2026-10-02T23:55:00+00:00",
                "boot_id": "boot",
                "machine_id": "machine"
            }
        });
        validate_injection(&v)?;
        Ok(())
    }

    #[test]
    fn same_boot_different_machine_cannot_certify_midnight() -> anyhow::Result<()> {
        let before = json!({
            "date": "2026-10-02",
            "boot_id": "boot",
            "machine_id": "machine"
        });
        let after = json!({
            "date": "2026-10-03",
            "boot_id": "boot",
            "machine_id": "other"
        });
        assert!(super::super::crossed(&before, &after).is_err());
        Ok(())
    }
}
