use super::{artifacts, Ssh};
use anyhow::{ensure, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::path::Path;
use std::time::{Duration, Instant};

const GUEST: &str = "/srv/qualification";
const MIB: usize = 1024 * 1024;
const DEADLINE: Duration = Duration::from_secs(20);
const MAX_ENCODED: usize = 9 * MIB;
const FILE_FUNCTION: &str = r#"emit_file() {
  label="$1"; path="$2"; cap="$3"
  if [ ! -d /srv/qualification ] || [ -L /srv/qualification ] || [ -L "$path" ] || { [ -e "$path" ] && [ ! -f "$path" ]; }; then
    printf '%s\tREFUSED\t0\t\t1\n' "$label"; return
  fi
  if [ ! -e "$path" ]; then printf '%s\tMISSING\t0\t\t1\n' "$label"; return; fi
  size=$(/usr/bin/stat -c %s -- "$path") || { printf '%s\tSTAT_FAILED\t0\t\t1\n' "$label"; return; }
  printf '%s\tOK\t%s\t' "$label" "$size"
  /usr/bin/tail -c "$cap" -- "$path" | /usr/bin/base64 -w0
  code=$?; printf '\t%s\n' "$code"
}
"#;
const SHOW: &str = "/usr/bin/systemctl show qualification.service --property=LoadState --property=ActiveState --property=SubState --property=Result --property=MainPID --property=ExecMainStatus --property=ExecMainCode --property=NRestarts --property=FragmentPath --no-pager";
const JOURNAL: &str =
    "/usr/bin/journalctl -b -u qualification.service --no-pager --output=short-iso --lines=200";

struct Spec {
    label: &'static str,
    source: Source,
    cap: usize,
}

enum Source {
    File(String),
    Command(&'static str),
}

impl Spec {
    fn file(label: &'static str, path: impl Into<String>, cap: usize) -> Self {
        Self {
            label,
            source: Source::File(path.into()),
            cap,
        }
    }
}

pub(super) fn collect(root: &Path, ssh: &Ssh, phase: &'static str) -> Result<Value> {
    ensure!(
        matches!(phase, "before-drain" | "after-drain"),
        "invalid diagnostics phase"
    );
    let deadline = Instant::now()
        .checked_add(DEADLINE)
        .context("diagnostics deadline overflow")?;
    let owner_spec = [Spec::file(
        "owner",
        format!("{GUEST}/current-owner.json"),
        16 * 1024,
    )];
    let owner = capture(root, ssh, phase, &owner_spec, deadline);
    let mut specs = fixed_specs();
    let selected = owner
        .as_ref()
        .map_err(|error| anyhow::anyhow!("{error:#}"))
        .and_then(|_| artifacts::json(&root.join(format!("diagnostics-{phase}-owner.bin"))))
        .and_then(|owner| selected_specs(&owner));
    if let Ok(selected) = &selected {
        specs.try_reserve_exact(selected.len())?;
        for spec in selected {
            let Source::File(path) = &spec.source else {
                anyhow::bail!("selected log is not a file");
            };
            specs.push(Spec::file(spec.label, path.clone(), spec.cap));
        }
    }
    let captured = capture(root, ssh, phase, &specs, deadline);
    let evidence = json!({"phase":phase,"observation_deadline_seconds":DEADLINE.as_secs(),
        "deadline_cleanup_policy":"Existing Process::stop may separately deliver TERM and use its bounded reap budget after this observation deadline; no SIGKILL",
        "owner":diagnostic(&owner),"owner_log_selection":selected.as_ref().err().map(|error|format!("{error:#}")),
        "selected":diagnostic(&captured),"snapshot_is_atomic":false,
        "file_sizes_observed_before_tails":true,"journal_entry_limit":200,
        "raw_tail_byte_limit":6_529_024,"encoded_batch_byte_limit":MAX_ENCODED,
        "collection_success":owner.as_ref().is_ok_and(all_read) && selected.is_ok() && captured.as_ref().is_ok_and(all_read),
        "at":artifacts::now()});
    artifacts::publish(&root.join(format!("diagnostics-{phase}.json")), &evidence)?;
    Ok(evidence)
}

fn fixed_specs() -> Vec<Spec> {
    vec![
        Spec::file("http", format!("{GUEST}/http.jsonl"), MIB),
        Spec::file("boots", format!("{GUEST}/boots.jsonl"), 64 * 1024),
        Spec::file("drain", format!("{GUEST}/current-drain.json"), 128 * 1024),
        Spec::file(
            "unit",
            "/etc/systemd/system/qualification.service",
            8 * 1024,
        ),
        Spec {
            label: "systemd",
            source: Source::Command(SHOW),
            cap: 16 * 1024,
        },
        Spec {
            label: "journal",
            source: Source::Command(JOURNAL),
            cap: MIB,
        },
    ]
}

fn selected_specs(owner: &Value) -> Result<Vec<Spec>> {
    let mut specs = Vec::new();
    specs.try_reserve_exact(4)?;
    for (field, label, out, err) in [
        ("endpoint", "serve", "endpoint-out", "endpoint-err"),
        ("node", "restate", "node-out", "node-err"),
    ] {
        let identity = owner
            .get(field)
            .context("selected process identity absent")?;
        let generation = identity
            .get("generation")
            .and_then(Value::as_str)
            .context("selected process generation absent")?;
        ensure!(
            valid_generation(generation),
            "selected process generation invalid"
        );
        ensure!(
            identity.get("label").and_then(Value::as_str) == Some(label),
            "selected process label differs"
        );
        ensure!(
            identity
                .get("pid")
                .and_then(Value::as_u64)
                .is_some_and(|pid| pid > 1),
            "selected process pid invalid"
        );
        let log = format!("{GUEST}/{label}-{generation}.log");
        ensure!(
            identity.get("log").and_then(Value::as_str) == Some(log.as_str()),
            "selected process log differs"
        );
        specs.push(Spec::file(out, log, MIB));
        specs.push(Spec::file(
            err,
            format!("{GUEST}/{label}-{generation}.stderr.log"),
            MIB,
        ));
    }
    Ok(specs)
}

fn valid_generation(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

fn capture(
    root: &Path,
    ssh: &Ssh,
    phase: &str,
    specs: &[Spec],
    deadline: Instant,
) -> Result<Value> {
    ensure!(
        !specs.is_empty() && specs.len() <= 10,
        "diagnostic file count exceeds bound"
    );
    let mut script = String::from(FILE_FUNCTION);
    for spec in specs {
        match &spec.source {
            Source::File(path) => script.push_str(&format!("emit_file {} {} {}\n", quote(spec.label), quote(path), spec.cap)),
            Source::Command(command) => script.push_str(&format!(
                "printf '%s\\tOK\\tunknown\\t' {}; {{ {}; }} | /usr/bin/tail -c {} | /usr/bin/base64 -w0; code=$?; printf '\\t%s\\n' \"$code\"\n",
                quote(spec.label), command, spec.cap)),
        }
    }
    let label = format!(
        "diagnostics-{phase}-{}",
        if specs.len() == 1 { "owner" } else { "logs" }
    );
    let remote = format!(
        "/usr/bin/timeout --signal=TERM 8s /usr/bin/bash -o pipefail -c {}",
        quote(&script)
    );
    let raw = ssh.exec_until(&label, &remote, deadline)?;
    ensure!(
        raw.len() <= MAX_ENCODED,
        "encoded diagnostic batch exceeds bound"
    );
    let lines = raw
        .lines()
        .take(specs.len().saturating_add(1))
        .collect::<Vec<_>>();
    ensure!(lines.len() == specs.len(), "diagnostic frame count differs");
    let mut retained = Vec::new();
    retained.try_reserve_exact(specs.len())?;
    for (spec, line) in specs.iter().zip(lines) {
        retained.push(retain(root, phase, spec, line)?);
    }
    Ok(json!(retained))
}

fn retain(root: &Path, phase: &str, spec: &Spec, line: &str) -> Result<Value> {
    let fields = line.split('\t').take(6).collect::<Vec<_>>();
    let [label, status, source_bytes, encoded, exit] = fields.as_slice() else {
        anyhow::bail!("diagnostic frame shape invalid");
    };
    ensure!(*label == spec.label, "diagnostic frame label differs");
    ensure!(
        ["OK", "REFUSED", "MISSING", "STAT_FAILED"].contains(status),
        "diagnostic frame status invalid"
    );
    let maximum = spec
        .cap
        .div_ceil(3)
        .checked_mul(4)
        .context("base64 bound overflow")?;
    ensure!(
        encoded.len() <= maximum,
        "diagnostic encoded file exceeds bound"
    );
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(spec.cap)?;
    bytes.resize(spec.cap, 0);
    let count = STANDARD.decode_slice(encoded, &mut bytes)?;
    bytes.truncate(count);
    let original = if *source_bytes == "unknown" {
        None
    } else {
        Some(source_bytes.parse::<u64>()?)
    };
    let code = exit.parse::<u32>()?;
    let path = root.join(format!("diagnostics-{phase}-{}.bin", spec.label));
    artifacts::write(&path, &bytes)?;
    Ok(
        json!({"label":label,"source":match &spec.source { Source::File(path)=>path.as_str(),Source::Command(command)=>command },
        "status":status,"exit_code":code,"file":path,"bytes":bytes.len(),"sha256":artifacts::sha(&bytes),
        "source_bytes_before_tail":original,"tail_limit_bytes":spec.cap,
        "tail_may_be_partial":original.is_some_and(|length|length > spec.cap as u64) || bytes.len() == spec.cap,
        "successful":*status == "OK" && code == 0}),
    )
}

fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn diagnostic(result: &Result<Value>) -> Value {
    json!({"evidence":result.as_ref().ok(),"failure":result.as_ref().err().map(|error|format!("{error:#}"))})
}

fn all_read(value: &Value) -> bool {
    value.as_array().is_some_and(|rows| {
        !rows.is_empty()
            && rows
                .iter()
                .all(|row| row.get("successful").and_then(Value::as_bool) == Some(true))
    })
}

#[cfg(test)]
mod tests;
