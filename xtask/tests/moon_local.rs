#![forbid(unsafe_code)]

#[macro_use]
#[path = "../../tools/fallible_checks.rs"]
mod fallible_checks;
#[path = "moon_local/fixture.rs"]
mod fixture;

use fixture::{executable, Fixture, TestResult};
use std::os::unix::fs::{symlink, MetadataExt};

fn check_launch(fixture: &Fixture, output: &std::process::Output, host: &str) -> TestResult {
    check!(eq; output.status.code(), Some(37), "{}", String::from_utf8_lossy(&output.stderr));
    check!(output.stderr.is_empty());
    let cc = fixture.bin.join("cc").display().to_string();
    let linux = if host == "x86_64-unknown-linux-gnu" {
        cc.as_str()
    } else {
        ""
    };
    let arm = if host == "aarch64-apple-darwin" {
        cc.as_str()
    } else {
        ""
    };
    let intel = if host == "x86_64-apple-darwin" {
        cc.as_str()
    } else {
        ""
    };
    check!(eq; String::from_utf8(output.stdout.clone())?, format!(
        "{}\0{}\0{}\0\0true\0{linux}\0{arm}\0{intel}\0",
        fixture.root.display(), fixture.bin.join("cargo").display(), fixture.bin.join("rustc").display()
    ));
    Ok(())
}

#[test]
fn linux_fingerprint_preserves_tools_wrappers_configs_and_linker() -> TestResult {
    let mut fixture = Fixture::new("x86_64-unknown-linux-gnu")?;
    executable(
        &fixture.bin.join("sha256sum"),
        "#!/usr/bin/env bash\nprintf 'used\\n' >> \"$HASH_LOG\"\nexec shasum -a 256 \"$@\"\n",
    )?;
    let wrapper = fixture.bin.join("workspace-wrapper");
    let hash_log = fixture.root.join("hash.log");
    fixture
        .command
        .env("HASH_LOG", &hash_log)
        .env("RUSTC_WORKSPACE_WRAPPER", &wrapper)
        .env("RUSTC_WRAPPER", "not-installed");
    let output = fixture.run()?;
    check_launch(&fixture, &output, "x86_64-unknown-linux-gnu")?;
    let mut hashes = Vec::new();
    for name in ["cargo", "rustc", "cargo-nextest", "moon", "cc", "c++", "ar"] {
        hashes.push(fixture.checksum_line(&fixture.bin.join(name))?);
    }
    hashes.push(fixture.checksum_line(&fixture.linker)?);
    check!(eq; fixture.fingerprint()?, serde_json::json!({
        "source_root": fixture.root,
        "cargo": "cargo fixture",
        "rustc": "rustc fixture\nhost: x86_64-unknown-linux-gnu",
        "nextest": "cargo-nextest fixture",
        "moon": "moon fixture",
        "cc": "cc fixture",
        "cxx": "c++ fixture",
        "ar": "GNU ar fixture",
        "executables": hashes.join("\n"),
        "wrappers": format!("{}\n", fixture.checksum_line(&wrapper)?),
        "host_native_features": "target_feature=\"fixture\"",
        "cargo_build_config": "{\"build\":{}}",
        "cargo_target_config": "{\"target\":{}}",
        "cargo_profile_config": "{\"profile\":{}}"
    }));
    check!(eq; std::fs::read_to_string(hash_log)?, "used\nused\n");
    Ok(())
}

#[test]
fn apple_hosts_need_neither_gnu_utilities_nor_linux_bundled_linker() -> TestResult {
    for host in ["aarch64-apple-darwin", "x86_64-apple-darwin"] {
        let mut fixture = Fixture::new(host)?;
        let output = fixture.run()?;
        check_launch(&fixture, &output, host)?;
        let fingerprint = fixture.fingerprint()?;
        let executables = fingerprint
            .get("executables")
            .and_then(|value| value.as_str())
            .ok_or("executable hashes are missing")?;
        check!(executables.contains(&fixture.checksum_line(&fixture.linker)?));
        check!(!executables.contains("x86_64-unknown-linux-gnu"));
        check!(eq; fingerprint.get("ar").and_then(|value| value.as_str()),
            Some("Apple ar has no version option; identity is its executable SHA256"));
        check!(!fixture.bin.join("realpath").exists());
        check!(!fixture.bin.join("sha256sum").exists());
    }
    Ok(())
}

#[test]
fn apple_system_shims_resolve_to_selected_developer_tools() -> TestResult {
    let mut fixture = Fixture::new("aarch64-apple-darwin")?;
    fixture
        .command
        .env("CC", "/usr/bin/cc")
        .env("CXX", "/usr/bin/c++")
        .env("AR", "/usr/bin/ar")
        .env("FIXTURE_APPLE_AR", "selected-ar")
        .env("FIXTURE_LINKER", "/usr/bin/ld");
    let output = fixture.run()?;
    check!(eq; output.status.code(), Some(37), "{}", String::from_utf8_lossy(&output.stderr));
    let fingerprint = fixture.fingerprint()?;
    let hashes = fingerprint
        .get("executables")
        .and_then(|value| value.as_str())
        .ok_or("executable hashes are missing")?;
    for name in ["selected-cc", "selected-c++", "selected-ar", "ld"] {
        check!(hashes.contains(&fixture.checksum_line(&fixture.bin.join(name))?));
    }
    check!(!hashes.contains("/usr/bin/"));
    let stdout = String::from_utf8(output.stdout)?;
    check!(stdout.contains(&format!(
        "\0{}\0",
        fixture.bin.join("selected-cc").display()
    )));
    Ok(())
}

#[test]
fn unchanged_fingerprint_keeps_file_and_changed_linker_invalidates_it() -> TestResult {
    for host in ["x86_64-unknown-linux-gnu", "aarch64-apple-darwin"] {
        let mut fixture = Fixture::new(host)?;
        check!(eq; fixture.run()?.status.code(), Some(37));
        let path = fixture.root.join("tools/build-toolchain.json");
        let original = std::fs::read(&path)?;
        let metadata = std::fs::metadata(&path)?;
        check!(eq; fixture.run()?.status.code(), Some(37));
        check!(eq; std::fs::read(&path)?, original);
        check!(eq; std::fs::metadata(&path)?.ino(), metadata.ino());
        executable(&fixture.linker, "#!/usr/bin/env bash\nexit 19\n")?;
        check!(eq; fixture.run()?.status.code(), Some(37));
        check!(ne; std::fs::read(&path)?, original);
    }
    Ok(())
}

#[test]
fn missing_linker_refuses_moon_and_preserves_prior_fingerprint() -> TestResult {
    for host in ["x86_64-unknown-linux-gnu", "aarch64-apple-darwin"] {
        let mut fixture = Fixture::new(host)?;
        check!(eq; fixture.run()?.status.code(), Some(37));
        let original = fixture.fingerprint()?;
        std::fs::remove_file(&fixture.linker)?;
        let output = fixture.run()?;
        check!(eq; output.status.code(), Some(2));
        check!(output.stdout.is_empty());
        check!(String::from_utf8(output.stderr)?.contains("selected linker is not executable"));
        check!(eq; fixture.fingerprint()?, original);
    }
    Ok(())
}

#[test]
fn checksum_failure_does_not_fall_back_or_publish_a_fingerprint() -> TestResult {
    let mut fixture = Fixture::new("x86_64-unknown-linux-gnu")?;
    executable(
        &fixture.bin.join("sha256sum"),
        "#!/usr/bin/env bash\nprintf 'hash failed\\n' >&2\nexit 42\n",
    )?;
    let output = fixture.run()?;
    check!(eq; output.status.code(), Some(42));
    check!(output.stdout.is_empty());
    check!(eq; String::from_utf8(output.stderr)?, "hash failed\n");
    check!(!fixture.root.join("tools/build-toolchain.json").exists());
    Ok(())
}

#[test]
fn failed_driver_probe_with_valid_linker_output_preserves_prior_fingerprint() -> TestResult {
    let mut fixture = Fixture::new("aarch64-apple-darwin")?;
    check!(eq; fixture.run()?.status.code(), Some(37));
    let original = fixture.fingerprint()?;
    fixture.command.env("FIXTURE_LINKER_PROBE_EXIT", "46");
    let output = fixture.run()?;
    check!(eq; output.status.code(), Some(46));
    check!(output.stdout.is_empty());
    check!(eq; fixture.fingerprint()?, original);
    Ok(())
}

#[test]
fn failed_xcrun_probe_with_valid_archiver_output_preserves_prior_fingerprint() -> TestResult {
    for archiver in ["ar", "custom-ar"] {
        let mut fixture = Fixture::new("x86_64-apple-darwin")?;
        fixture.command.env("AR", fixture.bin.join(archiver));
        check!(eq; fixture.run()?.status.code(), Some(37));
        let original = fixture.fingerprint()?;
        fixture.command.env("FIXTURE_AR_PROBE_EXIT", "47");
        let output = fixture.run()?;
        check!(eq; output.status.code(), Some(47));
        check!(output.stdout.is_empty());
        check!(eq; fixture.fingerprint()?, original);
    }
    Ok(())
}

#[test]
fn nested_guard_and_gate_exception_preserve_literal_arguments() -> TestResult {
    let mut fixture = Fixture::new("aarch64-apple-darwin")?;
    fixture.command.env("MOON_TARGET", "pipeline:tests");
    let rejected = fixture.run()?;
    check!(eq; rejected.status.code(), Some(2));
    check!(rejected.stdout.is_empty());
    check!(String::from_utf8(rejected.stderr)?.contains("refusing a nested Moon run"));
    let args = ["run", "pipeline:fmt", "--", "a b", "", "$(false)"];
    fixture
        .command
        .env("MOON_TARGET", "pipeline:gate")
        .args(args);
    let permitted = fixture.run()?;
    check!(eq; permitted.status.code(), Some(37));
    check!(String::from_utf8(permitted.stdout)?.ends_with(&format!("{}\0", args.join("\0"))));
    Ok(())
}

#[test]
fn physical_root_and_explicit_rustc_and_archiver_are_fingerprinted() -> TestResult {
    let mut fixture = Fixture::new("x86_64-apple-darwin")?;
    let alias = fixture.root.with_file_name("repository alias");
    symlink(&fixture.root, &alias)?;
    fixture
        .command
        .current_dir(&alias)
        .env(
            "CDPATH",
            fixture
                .root
                .parent()
                .ok_or("repository parent is missing")?,
        )
        .env("RUSTC", fixture.bin.join("selected-rustc"))
        .env("AR", fixture.bin.join("custom-ar"));
    let output = fixture.run()?;
    check!(eq; output.status.code(), Some(37), "{}", String::from_utf8_lossy(&output.stderr));
    let fingerprint = fixture.fingerprint()?;
    check!(eq; fingerprint.get("source_root"), Some(&serde_json::json!(fixture.root)));
    check!(eq; fingerprint.get("ar").and_then(|value| value.as_str()), Some("custom ar fixture"));
    let hashes = fingerprint
        .get("executables")
        .and_then(|value| value.as_str())
        .ok_or("executable hashes are missing")?;
    check!(hashes.contains(&fixture.checksum_line(&fixture.bin.join("selected-rustc"))?));
    check!(hashes.contains(&fixture.checksum_line(&fixture.bin.join("custom-ar"))?));
    Ok(())
}

#[test]
fn configuration_errors_and_unknown_hosts_refuse_moon() -> TestResult {
    let mut configuration = Fixture::new("aarch64-apple-darwin")?;
    configuration
        .command
        .env("FIXTURE_CONFIG_ERROR", "configuration is malformed");
    let output = configuration.run()?;
    check!(eq; output.status.code(), Some(1));
    check!(output.stdout.is_empty());
    check!(eq; String::from_utf8(output.stderr)?, "configuration is malformed\n");
    let mut unknown = Fixture::new("not-a-supported-host")?;
    let output = unknown.run()?;
    check!(eq; output.status.code(), Some(2));
    check!(output.stdout.is_empty());
    check!(
        String::from_utf8(output.stderr)?.contains("unsupported rustc host: not-a-supported-host")
    );
    Ok(())
}
