use anyhow::Result;
use std::path::Path;

const DRIVER: &[(&str, &[u8])] = &[
    (
        "qualification_native_teams.rs",
        include_bytes!("../qualification_native_teams.rs"),
    ),
    ("mod.rs", include_bytes!("mod.rs")),
    ("artifacts.rs", include_bytes!("artifacts.rs")),
    ("captures.rs", include_bytes!("captures.rs")),
    ("config.rs", include_bytes!("config.rs")),
    ("evidence.rs", include_bytes!("evidence.rs")),
    ("evidence/action.rs", include_bytes!("evidence/action.rs")),
    (
        "evidence/action/tests.rs",
        include_bytes!("evidence/action/tests.rs"),
    ),
    (
        "evidence/action/progress.rs",
        include_bytes!("evidence/action/progress.rs"),
    ),
    ("evidence/parent.rs", include_bytes!("evidence/parent.rs")),
    ("evidence/source.rs", include_bytes!("evidence/source.rs")),
    ("evidence/faults.rs", include_bytes!("evidence/faults.rs")),
    (
        "evidence/faults/slots.rs",
        include_bytes!("evidence/faults/slots.rs"),
    ),
    (
        "evidence/faults/timeline.rs",
        include_bytes!("evidence/faults/timeline.rs"),
    ),
    (
        "evidence/faults/tests.rs",
        include_bytes!("evidence/faults/tests.rs"),
    ),
    ("http.rs", include_bytes!("http.rs")),
    ("isolation.rs", include_bytes!("isolation.rs")),
    ("lifecycle.rs", include_bytes!("lifecycle.rs")),
    ("ledger.rs", include_bytes!("ledger.rs")),
    ("process.rs", include_bytes!("process.rs")),
    ("proxy.rs", include_bytes!("proxy.rs")),
    ("proxy/hold.rs", include_bytes!("proxy/hold.rs")),
    ("scenario.rs", include_bytes!("scenario.rs")),
    (
        "scenario/snapshot.rs",
        include_bytes!("scenario/snapshot.rs"),
    ),
    ("scenario/source.rs", include_bytes!("scenario/source.rs")),
    ("scenario/faults.rs", include_bytes!("scenario/faults.rs")),
    (
        "scenario/faults/interruption.rs",
        include_bytes!("scenario/faults/interruption.rs"),
    ),
    ("sources.rs", include_bytes!("sources.rs")),
];

const PRODUCTION: &[(&str, &[u8])] = &[
    (
        "production/team_source.rs",
        include_bytes!("../../src/restate_services/jurisdiction/team_source.rs"),
    ),
    (
        "production/team_source/ledger.rs",
        include_bytes!("../../src/restate_services/jurisdiction/team_source/ledger.rs"),
    ),
    (
        "production/team_source/admission.rs",
        include_bytes!("../../src/restate_services/jurisdiction/team_source/admission.rs"),
    ),
    (
        "production/team_source/ledger/history.rs",
        include_bytes!("../../src/restate_services/jurisdiction/team_source/ledger/history.rs"),
    ),
    (
        "production/team_source/ledger/attempt.rs",
        include_bytes!("../../src/restate_services/jurisdiction/team_source/ledger/attempt.rs"),
    ),
    (
        "production/team_source/ledger/settlement.rs",
        include_bytes!("../../src/restate_services/jurisdiction/team_source/ledger/settlement.rs"),
    ),
    (
        "production/team_collection.rs",
        include_bytes!("../../src/restate_services/jurisdiction/team_collection.rs"),
    ),
    (
        "production/teams_sources.rs",
        include_bytes!("../../src/restate_services/wire/teams_sources.rs"),
    ),
    (
        "production/teams.rs",
        include_bytes!("../../src/restate_services/wire/teams.rs"),
    ),
];

pub fn save(root: &Path) -> Result<()> {
    let path = root.join("driver-sources");
    std::fs::create_dir(&path)?;
    [
        "evidence",
        "evidence/action",
        "evidence/faults",
        "production",
        "production/team_source",
        "production/team_source/ledger",
        "scenario",
        "scenario/faults",
        "proxy",
    ]
    .iter()
    .try_for_each(|directory| std::fs::create_dir(path.join(directory)))?;
    DRIVER
        .iter()
        .chain(PRODUCTION)
        .try_for_each(|(name, body)| super::artifacts::write_new(&path.join(name), body))
}
