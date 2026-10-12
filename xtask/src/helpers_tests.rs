use super::lists_a_test;

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
