use super::super::super::process;
use super::*;

mod ownership;

fn baseline() -> Result<(Value, Value, Value)> {
    let capture: Value = serde_json::from_str(include_str!("tests/root10.json"))?;
    Ok((
        capture["injection"].clone(),
        capture["unfinished_sleep_witness"].clone(),
        capture["baseline"].clone(),
    ))
}

#[test]
fn authentic_v2_sleep_certifies_the_measured_pre_midnight_baseline() -> Result<()> {
    let (injection, active, clock) = baseline()?;
    let proof = validate_clock_baseline(&injection, &active, &clock)?;
    assert_eq!(proof["completion_id"], json!(2));
    assert_eq!(proof["wake_up_time"], json!(1790988900497_u64));
    assert_eq!(proof["guest_now"], json!(1790985302218_u64));
    Ok(())
}

#[test]
fn missed_midnight_is_rejected_instead_of_waiting_for_another_day() -> Result<()> {
    let (injection, active, mut clock) = baseline()?;
    clock["date"] = json!("2026-10-03");
    assert!(validate_clock_baseline(&injection, &active, &clock).is_err());
    Ok(())
}

#[test]
fn setup_reboot_cannot_certify_a_clock_only_boundary() -> Result<()> {
    let (injection, active, mut clock) = baseline()?;
    clock["boot_id"] = json!("reboot");
    assert!(validate_clock_baseline(&injection, &active, &clock).is_err());
    Ok(())
}

#[test]
fn completed_sleep_cannot_certify_the_pre_midnight_baseline() -> Result<()> {
    let (injection, mut active, clock) = baseline()?;
    let observation = &mut active["boundary"]["observation"];
    let id = observation["id"].clone();
    observation["journal"]["rows"]
        .as_array_mut()
        .context("captured journal absent")?
        .push(json!({
            "id":id, "index":4, "version":2, "entry_type":"Notification: Sleep",
            "entry_json":json!({"Notification":{"Completion":{"Sleep":{"completion_id":2}}}}).to_string()
        }));
    for side in ["before", "after"] {
        observation[side]["rows"][0]["journal_size"] = json!(5);
    }
    assert!(validate_clock_baseline(&injection, &active, &clock).is_err());
    Ok(())
}

#[test]
fn cached_proof_cannot_override_a_different_actual_awaited_completion() -> Result<()> {
    let (injection, mut active, clock) = baseline()?;
    for side in ["before", "after"] {
        active["boundary"]["observation"][side]["rows"][0]["suspended_waiting_future_json"] =
            json!(json!({"Single":{"CompletionId":7}}).to_string());
    }
    assert!(validate_clock_baseline(&injection, &active, &clock).is_err());
    Ok(())
}

#[test]
fn clock_sleep_must_belong_to_the_acknowledged_original_invocation() -> Result<()> {
    for field in ["id", "key", "accepted"] {
        let (injection, mut active, clock) = baseline()?;
        if field == "accepted" {
            active["invocation"]["accepted"]["invocationId"] = json!("inv_other");
        } else {
            active["invocation"][field] = json!("other");
        }
        assert!(validate_clock_baseline(&injection, &active, &clock).is_err());
    }
    Ok(())
}

#[test]
fn an_independently_valid_foreign_workflow_cannot_certify_the_clock_run() -> Result<()> {
    let (injection, mut active, clock) = baseline()?;
    active["invocation"]["key"] = json!("other_clock_workflow");
    let observation = &mut active["boundary"]["observation"];
    observation["key"] = json!("other_clock_workflow");
    for side in ["before", "after"] {
        observation[side]["rows"][0]["target_service_key"] = json!("other_clock_workflow");
    }
    let proof = super::super::super::native::sleep::unfinished(observation, &clock)?
        .context("foreign workflow has no internally valid Sleep witness")?;
    assert_eq!(proof["completion_id"], json!(2));
    assert!(validate_clock_baseline(&injection, &active, &clock).is_err());
    Ok(())
}

#[test]
fn crossing_requires_exactly_the_next_date_and_the_same_boot() -> Result<()> {
    let before = json!({"date":"2026-10-02","boot_id":"boot","machine_id":"machine"});
    assert!(!crossed(&before, &before)?);
    assert!(crossed(
        &before,
        &json!({"date":"2026-10-03","boot_id":"boot","machine_id":"machine"})
    )?);
    assert!(crossed(
        &before,
        &json!({"date":"2026-10-04","boot_id":"boot","machine_id":"machine"})
    )
    .is_err());
    Ok(())
}

#[test]
fn stalled_ssh_started_at_329_seconds_expires_and_retains_separate_reap() -> Result<()> {
    let root = tempfile::tempdir()?;
    let stalled = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
    let ssh = Ssh {
        root: root.path().into(),
        port: stalled.local_addr()?.port(),
    };
    let before = json!({
        "date": "2026-10-02",
        "boot_id": "boot",
        "machine_id": "machine",
        "realtime": "2026-10-02T23:55:02Z",
        "monotonic_uptime": "100.00 0.00"
    });
    let now = Instant::now();
    let started = now
        .checked_sub(Duration::from_secs(329))
        .context("test clock underflow")?;
    let deadline = started
        .checked_add(Duration::from_secs(330))
        .context("test deadline overflow")?;
    let error = cross_midnight(&ssh, &before, deadline)
        .err()
        .context("stalled observation accepted")?;
    assert!(
        error.downcast_ref::<process::DeadlineExpired>().is_some(),
        "{error:#}"
    );
    assert!(
        now.elapsed() < Duration::from_secs(5),
        "observation used generic SSH budget: {error:#}"
    );
    verify_ssh_reap(root.path())
}

fn verify_ssh_reap(root: &Path) -> Result<()> {
    let bytes = artifacts::read(&root.join("processes.jsonl"))?;
    let events = std::str::from_utf8(&bytes)?
        .lines()
        .map(serde_json::from_str::<Value>)
        .collect::<serde_json::Result<Vec<_>>>()?;
    let reap = events
        .iter()
        .find(|event| {
            event.get("event") == Some(&json!("reap"))
                && event.pointer("/evidence/identity/label") == Some(&json!("clock"))
        })
        .context("timed-out SSH reap absent")?;
    assert_eq!(reap.pointer("/evidence/reaped"), Some(&json!(true)));
    assert_eq!(reap.pointer("/evidence/requested_signal"), Some(&json!(15)));
    assert_eq!(reap.pointer("/evidence/exit/signal"), Some(&json!(15)));
    let pid = reap
        .pointer("/evidence/identity/pid")
        .and_then(Value::as_u64)
        .context("SSH PID absent")?;
    assert!(!Path::new(&format!("/proc/{pid}")).exists());
    Ok(())
}

#[test]
fn expired_clock_observation_never_launches_a_child() -> Result<()> {
    let root = tempfile::tempdir()?;
    let ssh = Ssh {
        root: root.path().into(),
        port: 1,
    };
    let error = ssh
        .clock_until(Instant::now())
        .err()
        .context("expired clock accepted")?;
    assert!(error.downcast_ref::<process::DeadlineExpired>().is_some());
    assert!(!root.path().join("processes.jsonl").exists());
    Ok(())
}
