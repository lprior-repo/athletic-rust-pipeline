mod coach_directories;
mod ihsa;
mod ihsa_tournament;
mod ks;
mod mshsl;
mod ohsaa;
mod plain_names;
mod results;
mod wayzata;
mod wiaa;

use crate::replay::Capture;
use anyhow::{bail, Result};

pub(super) fn replay(capture: &Capture<'_>) -> Result<String> {
    match capture.source {
        "wiaa" => wiaa::wiaa(capture),
        "mshsl" => mshsl::mshsl(capture),
        "ohsaa" => ohsaa::ohsaa(capture),
        "plain_names" => plain_names::plain_names(capture),
        "ihsa" => ihsa::ihsa(capture),
        "ihsa_tournament" => ihsa_tournament::ihsa_tournament(capture),
        "ks" => ks::ks_directory(capture),
        "wayzata" => wayzata::wayzata_schedule(capture),
        "coach_directories" => coach_directories::coach_directories(capture),
        "milesplit"
        | "athleticlive"
        | "athleticlive_athletes"
        | "athleticnet"
        | "athleticlive_results"
        | "tfrrs"
        | "wiaa_results" => results::replay(capture),
        other => bail!("no replay arm for source `{other}`: xtask/src/replay/cases.rs states them"),
    }
}
