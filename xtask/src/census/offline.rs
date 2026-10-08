use anyhow::Result;
use census_report::report::Scope;
use std::path::Path;

use crate::cmd::Cmd;

use super::ExportRequest;

pub(super) fn report(store: &Path, scope: Scope) -> Result<()> {
    let mut cmd = binary(store).arg("report");
    if let Scope::Core = scope {
        cmd = cmd.arg("--core");
    }
    cmd.run()
}

pub(super) fn workbook(store: &Path, request: &ExportRequest) -> Result<()> {
    let mut cmd = binary(store).args([
        "workbook",
        "--grad-year",
        &request.grad_year.to_string(),
        "--school-year",
        &request.school_year.to_string(),
    ]);
    if let Some(out) = request.out.as_deref() {
        cmd = cmd.arg("--out").arg(out.display().to_string());
    }
    if request.core {
        cmd = cmd.arg("--core");
    }
    if let Some(limit) = request.limit {
        cmd = cmd.args(["--limit", &limit.to_string()]);
    }
    cmd.run()
}

fn binary(store: &Path) -> Cmd {
    Cmd::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "census-service",
            "--bin",
            "census-service",
            "--",
            "--store",
        ])
        .arg(store.display().to_string())
}
