use super::{BTreeMap, BTreeSet, Command, Output, Path, Table, CENSUS_BIN, DEAD_PROXY};

pub(super) fn census_command(root: &Path) -> Command {
    let mut command = Command::new(CENSUS_BIN);
    command
        .arg("--store")
        .arg(root)
        .arg("--delay-ms")
        .arg("0")
        .env("RUST_LOG", "off")
        .env("HTTPS_PROXY", DEAD_PROXY)
        .env("HTTP_PROXY", DEAD_PROXY);
    command
}

pub(super) fn run_census(root: &Path, args: &[&str]) -> super::TestResult<Output> {
    let output = census_command(root).args(args).output()?;
    check!(
        output.status.success(),
        "census-service {args:?} failed with {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output)
}

pub(super) fn report_of(output: &Output) -> super::TestResult<serde_json::Value> {
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(serde_json::from_str(&stdout)?)
}

pub(super) fn counters_of(output: &Output) -> BTreeMap<String, u64> {
    let mut allowed: BTreeSet<&str> = Table::ALL.iter().map(|table| table.file()).collect();
    allowed.insert("observations");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .filter(|(name, _)| allowed.contains(*name))
        .filter_map(|(name, count)| {
            count
                .trim()
                .parse::<u64>()
                .ok()
                .map(|count| (name.to_string(), count))
        })
        .collect()
}
