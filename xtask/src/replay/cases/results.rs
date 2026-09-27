
use crate::replay::Capture;
use anyhow::{bail, Result};

mod athleticlive;
mod athleticnet;
mod milesplit;
mod tfrrs;
mod wiaa;

pub(super) fn replay(capture: &Capture<'_>) -> Result<String> {
    match capture.source {
        "wiaa_results" => wiaa::result_file(capture),
        "athleticlive_athletes" => athleticlive::athlete_hits(capture),
        "milesplit" => milesplit::capture(capture),
        "tfrrs" => tfrrs::capture(capture),
        "athleticlive_results" => athleticlive::event_document(capture),
        "athleticlive" => athleticlive::meets_csv(capture),
        "athleticnet" => athleticnet::document(capture),
        other => bail!("no result-plane replay arm for source `{other}`"),
    }
}
