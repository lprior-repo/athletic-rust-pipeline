use super::lists_a_test;
use std::os::unix::fs::PermissionsExt;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn a_listing_counts_only_actual_tests() -> TestResult {
    check!(
        lists_a_test("chsaa::tests::directory_parses_the_378_member_schools: test\n"),
        "a libtest line for one test is a match"
    );
    check!(
        lists_a_test("a: test\nb: test\n"),
        "any matching test line is a match"
    );
    check!(
        !lists_a_test("chsaa::benches::directory_parse: benchmark\n"),
        "a benchmark line is not a test"
    );
    check!(
        !lists_a_test("0 tests, 0 benchmarks\n"),
        "an empty listing is not a match"
    );
    check!(!lists_a_test(""), "an empty capture is not a match");
    Ok(())
}

#[test]
fn gate_lane_selects_only_contract_and_preserves_failure() -> TestResult {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/gate.sh");
    for (exit, status, verdict) in [
        ("0", 0, "architecture contract: PASS"),
        ("37", 1, "architecture contract: FAIL"),
    ] {
        let output = std::process::Command::new("bash")
            .args([
                "-c",
                "cargo() { printf 'selected-cargo:%s\\n' \"$*\"; return \"$LANE_EXIT\"; }; export -f cargo; bash \"$GATE_SCRIPT\" --release --lane contract",
            ])
            .env("GATE_SCRIPT", &script)
            .env("LANE_EXIT", exit)
            .output()?;
        let stdout = String::from_utf8(output.stdout)?;
        let stderr = String::from_utf8(output.stderr)?;
        check!(eq; output.status.code(), Some(status), "{stdout}\n{stderr}");
        check!(stdout.contains("selected-cargo:run -q -p xtask -- contract"));
        check!(stdout.contains(verdict));
        check!(eq; stdout.matches("selected-cargo:").count(), 1);
    }
    Ok(())
}

#[test]
fn gate_lane_rejects_invalid_selection_before_running_checks() -> TestResult {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/gate.sh");
    for (args, diagnostic) in [
        ("--lane", "exactly one lane name is required"),
        ("--lane absent", "unknown lane: absent"),
        (
            "--lane contract --lane scan",
            "exactly one lane name is required",
        ),
        (
            "--lane contract --update-baseline",
            "--lane cannot be combined with baseline updates",
        ),
        (
            "--lane contract --allow-increase",
            "--lane cannot be combined with baseline updates",
        ),
    ] {
        let output = std::process::Command::new("bash")
            .args([
                "-c",
                "cargo() { printf 'unexpected-cargo\\n'; return 0; }; export -f cargo; bash \"$GATE_SCRIPT\" \"$@\"",
                "gate-test",
            ])
            .args(args.split_ascii_whitespace())
            .env("GATE_SCRIPT", &script)
            .output()?;
        let stdout = String::from_utf8(output.stdout)?;
        let stderr = String::from_utf8(output.stderr)?;
        check!(eq; output.status.code(), Some(2), "{args}: {stdout}\n{stderr}");
        check!(stderr.contains(diagnostic), "{args}: {stderr}");
        check!(eq; stdout, "", "{args}");
    }
    Ok(())
}

fn durability_guard(
    admin: &str,
    service: &str,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/gate.sh");
    let empty = tempfile::tempdir()?;
    Ok(std::process::Command::new("bash")
        .args([
            "-c",
            "source \"$GATE_SCRIPT\" && cd \"$EMPTY_DIR\" && RELEASE=1 lane_durability",
        ])
        .env("GATE_SCRIPT", &script)
        .env("EMPTY_DIR", empty.path())
        .env(
            "RESTATE_SERVER_BIN",
            empty.path().join("missing-restate-server"),
        )
        .env("ADMIN_PORT", admin)
        .env("SERVICE_PORT", service)
        .output()?)
}

#[test]
fn durability_guard_refuses_shared_malformed_and_duplicate_ports_before_resources() -> TestResult {
    for (admin, service, diagnostic) in [
        ("19095", "9405", "durability: forbidden port: 19095"),
        ("19405", "9080", "durability: forbidden port: 9080"),
        ("18095", "9405", "durability: forbidden port: 18095"),
        ("15192", "9405", "durability: forbidden port: 15192"),
        ("abc", "9405", "durability: invalid port: abc"),
        ("65536", "9405", "durability: invalid port: 65536"),
        ("0", "9405", "durability: invalid port: 0"),
        ("19405", "19405", "durability: duplicate ports: 19405"),
    ] {
        let output = durability_guard(admin, service)?;
        let stdout = String::from_utf8(output.stdout)?;
        let stderr = String::from_utf8(output.stderr)?;
        check!(eq; output.status.code(), Some(1), "{stdout}\n{stderr}");
        check!(
            stderr.contains(diagnostic),
            "{admin}/{service}: {stdout}\n{stderr}"
        );
        check!(eq; stdout, "");
    }
    Ok(())
}

#[test]
fn durability_guard_refuses_an_occupied_port_before_resources() -> TestResult {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port().to_string();
    let output = durability_guard(&port, "9405")?;
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    check!(eq; output.status.code(), Some(1), "{stdout}\n{stderr}");
    check!(
        stderr.contains(&format!("durability: occupied port: {port}")),
        "{stdout}\n{stderr}"
    );
    check!(eq; stdout, "");
    Ok(())
}

struct GateCase {
    root: tempfile::TempDir,
    bin: std::path::PathBuf,
    json: std::path::PathBuf,
    log: std::path::PathBuf,
    exit: String,
    scan: String,
    ratchet: String,
}

fn repo_path(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn real_cargo() -> Result<String, Box<dyn std::error::Error>> {
    let path = std::env::var_os("PATH").ok_or("PATH is absent")?;
    cargo_in_path(&path)
        .map(|candidate| candidate.display().to_string())
        .ok_or_else(|| "no cargo in the original PATH".into())
}

fn cargo_in_path(path: &std::ffi::OsStr) -> Option<std::path::PathBuf> {
    std::env::split_paths(path)
        .map(|entry| entry.join("cargo"))
        .find(|candidate| candidate.is_file())
}

#[test]
fn gate_cargo_lookup_preserves_the_first_original_path_entry(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let first = root.path().join("first");
    let later = root.path().join("later");
    std::fs::create_dir(&first)?;
    std::fs::create_dir(&later)?;
    std::fs::write(first.join("cargo"), b"first cargo fixture")?;
    std::fs::write(later.join("cargo"), b"later cargo fixture")?;
    let path = std::env::join_paths([&first, &later])?;
    check!(eq; cargo_in_path(&path), Some(first.join("cargo")));
    let missing = std::env::join_paths([root.path().join("missing")])?;
    check!(eq; cargo_in_path(&missing), None);
    Ok(())
}

fn clippy_rows() -> Result<serde_json::Map<String, serde_json::Value>, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(repo_path("../tools/quality-baseline.json"))?;
    let value: serde_json::Value = serde_json::from_str(&text)?;
    Ok(value["clippy"]
        .as_object()
        .ok_or("the baseline records no clippy debt")?
        .clone())
}

fn clippy_json(
    rows: &serde_json::Map<String, serde_json::Value>,
    finish: Option<bool>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut lines = Vec::new();
    for (key, count) in rows {
        let (target, code) = key
            .split_once('\t')
            .ok_or("baseline clippy keys are `target<TAB>lint`")?;
        for _ in 0..count.as_u64().ok_or("baseline clippy counts are numbers")? {
            lines.push(
                serde_json::json!({
                    "reason": "compiler-message",
                    "target": { "name": target },
                    "message": {
                        "level": "error",
                        "code": { "code": code },
                        "rendered": "injected diagnostic"
                    }
                })
                .to_string(),
            );
        }
    }
    if let Some(success) = finish {
        lines.push(
            serde_json::json!({ "reason": "build-finished", "success": success }).to_string(),
        );
    }
    Ok(lines.join("\n"))
}

fn gate_case(payload: &str, exit: &str) -> Result<GateCase, Box<dyn std::error::Error>> {
    gate_case_with_exits(payload, exit, "1", "1")
}

fn gate_case_with_exits(
    payload: &str,
    clippy: &str,
    scan: &str,
    ratchet: &str,
) -> Result<GateCase, Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let bin = root.path().join("bin");
    std::fs::create_dir(&bin)?;
    let stub = bin.join("cargo");
    let real = real_cargo()?;
    std::fs::write(
        &stub,
        format!(
            "#!/usr/bin/env bash\nprintf '%s\\n' \"$*\" >> \"$STUB_LOG\"\n\
             for a in \"$@\"; do\n\
             \x20 if [ \"$a\" = clippy ]; then cat \"$STUB_JSON\"; exit \"$STUB_EXIT\"; fi\n\
             \x20 if [ \"$a\" = scan ]; then exit \"${{STUB_SCAN_EXIT:-1}}\"; fi\n\
             \x20 if [ \"$a\" = ratchet ]; then exit \"${{STUB_RATCHET_EXIT:-1}}\"; fi\n\
             done\n\
             exec {real} \"$@\"\n"
        ),
    )?;
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755))?;
    let json = root.path().join("clippy.json");
    std::fs::write(&json, payload)?;
    let log = root.path().join("cargo.log");
    std::fs::write(&log, "")?;
    Ok(GateCase {
        root,
        bin,
        json,
        log,
        exit: clippy.to_string(),
        scan: scan.to_string(),
        ratchet: ratchet.to_string(),
    })
}

fn run_sourced(
    case: &GateCase,
    script: &str,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(std::process::Command::new("bash")
        .args(["-c", script])
        .current_dir(case.root.path())
        .env("GATE_SCRIPT", repo_path("../tools/gate.sh"))
        .env("STUB_JSON", &case.json)
        .env("STUB_EXIT", &case.exit)
        .env("STUB_SCAN_EXIT", &case.scan)
        .env("STUB_RATCHET_EXIT", &case.ratchet)
        .env("STUB_LOG", &case.log)
        .env(
            "PATH",
            format!("{}:{}", case.bin.display(), std::env::var("PATH")?),
        )
        .output()?)
}

fn invocations(case: &GateCase) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(&case.log)?
        .lines()
        .map(str::to_string)
        .collect())
}

#[test]
fn ratchet_measurement_emits_a_tally_only_for_a_complete_success() -> TestResult {
    let rows = clippy_rows()?;
    let expected: std::collections::BTreeSet<String> = rows
        .iter()
        .map(|(key, count)| format!("{key}\t{count}"))
        .collect();
    let script = "source \"$GATE_SCRIPT\"\n\
                  tmp=$(mktemp -d)\n\
                  measure_clippy \"$tmp/clippy.json\" > \"$tmp/clippy.tsv\"\n\
                  status=$?\n\
                  sort \"$tmp/clippy.tsv\"\n\
                  exit $status";
    let complete = gate_case(&clippy_json(&rows, Some(true))?, "0")?;
    let output = run_sourced(&complete, script)?;
    let stdout = String::from_utf8(output.stdout)?;
    let measured: std::collections::BTreeSet<String> = stdout.lines().map(str::to_string).collect();
    check!(eq; output.status.code(), Some(0), "{stdout}");
    check!(eq; measured, expected, "{stdout}");

    for (payload, exit, diagnostic) in [
        (
            clippy_json(&rows, Some(false))?,
            "101",
            "strict clippy exited 101",
        ),
        (
            clippy_json(&rows, Some(false))?,
            "0",
            "did not report a successful build-finished record",
        ),
        (
            clippy_json(&rows, Some(true))?,
            "101",
            "strict clippy exited 101",
        ),
        (
            clippy_json(&rows, None)?,
            "0",
            "did not report a successful build-finished record",
        ),
        ("{\"reason\": \"compil".to_string(), "0", "did not report"),
    ] {
        let case = gate_case(&payload, exit)?;
        let output = run_sourced(&case, script)?;
        let stdout = String::from_utf8(output.stdout)?;
        let stderr = String::from_utf8(output.stderr)?;
        check!(
            output.status.code().is_some_and(|code| code != 0),
            "{stdout}\n{stderr}"
        );
        check!(stderr.contains(diagnostic), "{stdout}\n{stderr}");
        check!(eq; stdout.trim(), "", "{stdout}");
    }
    Ok(())
}

#[test]
fn ratchet_lane_never_compares_without_a_complete_measurement() -> TestResult {
    let rows = clippy_rows()?;
    let script = "source \"$GATE_SCRIPT\"\ntmp=$(mktemp -d)\nlane_ratchet \"$tmp\"";
    let broken = gate_case(&clippy_json(&rows, Some(false))?, "101")?;
    let output = run_sourced(&broken, script)?;
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    check!(
        output.status.code().is_some_and(|code| code != 0),
        "{stdout}\n{stderr}"
    );
    check!(
        stderr.contains("strict clippy exited 101"),
        "{stdout}\n{stderr}"
    );
    let attempted = invocations(&broken)?;
    check!(
        !attempted.iter().any(|line| line.contains("scan")),
        "{attempted:?}"
    );
    check!(
        !attempted.iter().any(|line| line.contains("ratchet")),
        "{attempted:?}"
    );

    let complete = gate_case(&clippy_json(&rows, Some(true))?, "0")?;
    let output = run_sourced(&complete, script)?;
    let stderr = String::from_utf8(output.stderr)?;
    check!(!stderr.contains("exited 101"), "{stderr}");
    check!(
        !stderr.contains("did not report a successful build-finished record"),
        "{stderr}"
    );
    let attempted = invocations(&complete)?;
    check!(
        attempted.iter().any(|line| line.contains("scan")),
        "{attempted:?}"
    );
    Ok(())
}

#[test]
fn ratchet_lane_refuses_a_failed_clippy_measurement_with_a_baseline_exact_tally() -> TestResult {
    let rows = clippy_rows()?;
    let payload = clippy_json(&rows, Some(false))?;
    let counted: u64 = rows.values().filter_map(serde_json::Value::as_u64).sum();
    let expected = usize::try_from(counted)?;
    let script = "source \"$GATE_SCRIPT\"\nLANE=ratchet\nselected_main";
    let case = gate_case_with_exits(&payload, "101", "0", "0")?;
    let output = run_sourced(&case, script)?;
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    check!(eq; output.status.code(), Some(1), "{stdout}\n{stderr}");
    check!(
        stderr.contains("strict clippy exited 101"),
        "{stdout}\n{stderr}"
    );
    check!(
        eq;
        stdout.matches("injected diagnostic").count(),
        expected,
        "{stdout}"
    );
    check!(stdout.contains("--- debt ratchet: FAIL"), "{stdout}");
    check!(stdout.contains("gate: FAIL -> debt ratchet"), "{stdout}");
    check!(!stdout.contains("PASS"), "{stdout}");
    let attempted = invocations(&case)?;
    check!(
        !attempted.iter().any(|line| line.contains("-- scan")),
        "{attempted:?}"
    );
    check!(
        !attempted.iter().any(|line| line.contains("-- ratchet")),
        "{attempted:?}"
    );
    Ok(())
}

#[test]
fn ratchet_lane_treats_malformed_completion_records_as_an_incomplete_measurement() -> TestResult {
    let rows = clippy_rows()?;
    let unnamed = format!(
        "{}\n{}",
        clippy_json(&rows, None)?,
        serde_json::json!({ "reason": "build-finished" })
    );
    let untyped = format!(
        "{}\n{}",
        clippy_json(&rows, None)?,
        serde_json::json!({ "reason": "build-finished", "success": "false" })
    );
    let truncated = format!("{}\n{{\"reason\": \"compil", clippy_json(&rows, Some(true))?);
    let script = "source \"$GATE_SCRIPT\"\ntmp=$(mktemp -d)\nlane_ratchet \"$tmp\"";
    for (payload, diagnostic) in [
        (
            unnamed.as_str(),
            Some("did not report a successful build-finished record"),
        ),
        (
            untyped.as_str(),
            Some("did not report a successful build-finished record"),
        ),
        (truncated.as_str(), None),
    ] {
        let case = gate_case_with_exits(payload, "0", "0", "0")?;
        let output = run_sourced(&case, script)?;
        let stdout = String::from_utf8(output.stdout)?;
        let stderr = String::from_utf8(output.stderr)?;
        check!(
            output.status.code().is_some_and(|code| code != 0),
            "{stdout}\n{stderr}"
        );
        check!(!stderr.contains("strict clippy exited"), "{stdout}\n{stderr}");
        if let Some(diagnostic) = diagnostic {
            check!(stderr.contains(diagnostic), "{stdout}\n{stderr}");
        }
        let attempted = invocations(&case)?;
        check!(
            !attempted.iter().any(|line| line.contains("-- scan")),
            "{attempted:?}"
        );
        check!(
            !attempted.iter().any(|line| line.contains("-- ratchet")),
            "{attempted:?}"
        );
    }
    Ok(())
}

#[test]
fn ratchet_lane_reaches_the_comparison_and_reports_its_verdict() -> TestResult {
    let rows = clippy_rows()?;
    let payload = clippy_json(&rows, Some(true))?;
    let script = "source \"$GATE_SCRIPT\"\nLANE=ratchet\nselected_main";
    for (comparison, status, lane, verdict) in [
        ("0", 0, "--- debt ratchet: PASS", "gate: PASS"),
        ("1", 1, "--- debt ratchet: FAIL", "gate: FAIL -> debt ratchet"),
    ] {
        let case = gate_case_with_exits(&payload, "0", "0", comparison)?;
        let output = run_sourced(&case, script)?;
        let stdout = String::from_utf8(output.stdout)?;
        let stderr = String::from_utf8(output.stderr)?;
        check!(eq; output.status.code(), Some(status), "{stdout}\n{stderr}");
        check!(stdout.contains(lane), "{comparison}: {stdout}");
        check!(stdout.contains(verdict), "{comparison}: {stdout}");
        check!(eq; stdout.contains("PASS"), status == 0, "{comparison}: {stdout}");
        let attempted = invocations(&case)?;
        check!(
            attempted.iter().any(|line| line.contains("-- scan")),
            "{attempted:?}"
        );
        let reached = attempted
            .iter()
            .find(|line| line.contains("-- ratchet"))
            .ok_or("the lane never reached the debt comparison")?;
        check!(reached.contains("tools/quality-baseline.json"), "{reached}");
        check!(reached.contains("clippy.tsv"), "{reached}");
        check!(reached.contains("scan.json"), "{reached}");
    }
    Ok(())
}

#[test]
fn baseline_update_refuses_an_incomplete_measurement() -> TestResult {
    let rows = clippy_rows()?;
    let root = tempfile::tempdir()?;
    let tools = root.path().join("tools");
    std::fs::create_dir(&tools)?;
    std::fs::copy(repo_path("../tools/gate.sh"), tools.join("gate.sh"))?;
    let baseline = tools.join("quality-baseline.json");
    std::fs::copy(repo_path("../tools/quality-baseline.json"), &baseline)?;
    let before = std::fs::read(&baseline)?;

    let case = gate_case(&clippy_json(&rows, Some(false))?, "101")?;
    let output = std::process::Command::new("bash")
        .arg(tools.join("gate.sh"))
        .arg("--update-baseline")
        .current_dir(root.path())
        .env("STUB_JSON", &case.json)
        .env("STUB_EXIT", &case.exit)
        .env("STUB_LOG", &case.log)
        .env(
            "PATH",
            format!("{}:{}", case.bin.display(), std::env::var("PATH")?),
        )
        .output()?;
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    check!(eq; output.status.code(), Some(1), "{stdout}\n{stderr}");
    check!(
        stdout.contains("refusing to refresh the baseline from an incomplete measurement"),
        "{stdout}\n{stderr}"
    );
    check!(!stdout.contains("baseline update: PASS"), "{stdout}");
    check!(eq; std::fs::read(&baseline)?, before);
    let attempted = invocations(&case)?;
    check!(
        !attempted
            .iter()
            .any(|line| line.contains("quality-baseline")),
        "{attempted:?}"
    );
    Ok(())
}
