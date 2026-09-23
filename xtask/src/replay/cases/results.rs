//! The result-plane replay arms: result-set routes and published documents.
//!
//! These captures are documents rather than site pages - a Hy-Tek or RaceDay result file, a
//! MileSplit index, roster or `/raw` body, the AthleticLIVE meets CSV and one of its event
//! documents, Athletic.net's published JSON, a TFRRS list or team page - and the arms call the same
//! entry points the adapters' fixture tests call, so this is the same read rather than a second
//! parser beside them.
//!
//! Two inputs a result file cannot state about itself are supplied the way the harnesses supply
//! them: its format, decided from its extension and body by the adapter's own classifier, and its
//! archive year, read from the fixture's own record under `tests/golden/`.
//!
//! One file per source family, beside this dispatcher: an arm reads the captures of exactly one
//! adapter, and a family's arms share the file-name grammar its captures are named by
//! ([`milesplit`], for instance, owns the roster and `/raw` name parsers its own arms resolve
//! captures with).

use crate::replay::Capture;
use anyhow::{bail, Result};

mod athleticlive;
mod athleticnet;
mod milesplit;
mod tfrrs;
mod wiaa;

/// Replay one result-plane capture, returning the line the verb prints for it.
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
