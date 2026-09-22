use crate::coachverify::verdict::{FragmentOutcome, RowOutcome};
use std::path::Path;

use super::fetch::verify_one_fragment;
use super::FragmentRow;

/// Read one fragment CSV: the 11 columns positionally, an optional 12th `verify` cell ignored.
pub fn read_fragment(path: &Path) -> anyhow::Result<Vec<FragmentRow>> {
    use anyhow::Context;
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(path)
        .with_context(|| format!("open fragment {path:?}"))?;
    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record.with_context(|| format!("read fragment row in {path:?}"))?;
        let cell = |index: usize| record.get(index).unwrap_or_default().trim().to_string();
        if cell(0) == "school" && cell(2) == "state" {
            continue;
        }
        let source_urls: Vec<String> = cell(9).split_whitespace().map(str::to_string).collect();
        rows.push(FragmentRow {
            school: cell(0),
            city: cell(1),
            state: cell(2),
            sport: cell(3),
            role: cell(4),
            coach_name: cell(5),
            public_professional_email: cell(6),
            ad_name: cell(7),
            ad_email: cell(8),
            source_urls,
            last_observed: cell(10),
        });
    }
    Ok(rows)
}

/// Write a verified fragment: the 11 columns plus the recomputed verdict.
pub fn write_fragment(path: &Path, outcomes: &[RowOutcome]) -> anyhow::Result<()> {
    use anyhow::Context;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create verified fragment dir {parent:?}"))?;
    }
    let mut writer = csv::WriterBuilder::new()
        .from_path(path)
        .with_context(|| format!("open verified fragment {path:?} for writing"))?;
    // Same shape as the input fragment: `merge-coaches` reads these columns and nothing else, and a
    // row that did not ship is simply absent. The verdict per row lives in `--csv`, per fragment in
    // the freeze log and manifest.
    writer.write_record(super::FRAGMENT_COLUMNS)?;
    for outcome in outcomes.iter().filter(|outcome| outcome.verdict.shipped()) {
        let row = &outcome.row;
        writer.write_record([
            row.school.as_str(),
            row.city.as_str(),
            row.state.as_str(),
            row.sport.as_str(),
            row.role.as_str(),
            row.coach_name.as_str(),
            row.public_professional_email.as_str(),
            row.ad_name.as_str(),
            row.ad_email.as_str(),
            row.source_urls.join(" ").as_str(),
            row.last_observed.as_str(),
        ])?;
    }
    writer.flush()?;
    Ok(())
}

/// The name a verified fragment keeps: the input directory becomes a prefix, so the two coach
/// fragment trees (`coach-fragments/` and `coach-fragments-ad/`) cannot overwrite each other's
/// same-named state files.
pub fn fragment_file_name(path: &Path) -> String {
    let file = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "fragment.csv".to_string());
    match path
        .parent()
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned())
    {
        Some(directory) if !directory.is_empty() => format!("{directory}-{file}"),
        _ => file,
    }
}

/// Run the whole gate over one fragment and write its verified copy.
pub async fn verify_fragment(
    fetcher: &crate::net::Fetcher,
    path: &Path,
    out_dir: &Path,
    options: &super::GateOptions,
) -> anyhow::Result<FragmentOutcome> {
    verify_one_fragment(fetcher, path, out_dir, options).await
}
