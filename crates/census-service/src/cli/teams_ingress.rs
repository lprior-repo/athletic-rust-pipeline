use anyhow::Result;
use census_crawl::{AdapterReport, CollectionDisposition as Disposition};
use census_service::restate_services::{JurisdictionReport, JurisdictionRequest};

use super::super::national::drive_jurisdiction;
use super::{teams_line, TeamsOutcomes};

#[tracing::instrument(skip_all)]
pub(super) async fn drive(
    origin: &str,
    requests: Vec<JurisdictionRequest>,
    rounds: u64,
    json: bool,
) -> Result<()> {
    let ingestion = census_service::ingress::client(origin)?;
    let mut outcomes = TeamsOutcomes::default();
    for request in requests {
        let jurisdiction = request.jurisdiction;
        match drive_jurisdiction(&ingestion, request, rounds).await {
            Ok(report) => {
                if !json {
                    println!("{}", teams_line(&report)?);
                }
                outcomes.record_report(jurisdiction, report_of(report)?);
            }
            Err(error) => {
                eprintln!("{}\tteams=error\t{error:#}", jurisdiction.code());
                let line = outcomes.record_interruption(jurisdiction, &error);
                if !json {
                    println!("{line}");
                }
            }
        }
    }
    if json {
        println!("{}", outcomes.json()?);
    }
    outcomes.finish()
}

fn report_of(report: JurisdictionReport) -> Result<AdapterReport> {
    let mut outcome = AdapterReport::new("jurisdiction", "teams");
    outcome.rows = u64::try_from(report.teams)?;
    outcome.notes = report.rosters.errors;
    outcome.unfinished = report
        .plan
        .refused
        .into_iter()
        .filter(|refusal| refusal.reason != "no run stage sweeps this source per jurisdiction")
        .map(|refusal| format!("{}: {}", refusal.slug, refusal.reason))
        .collect();
    outcome.errors = u64::try_from(outcome.notes.len())?
        .checked_add(u64::try_from(outcome.unfinished.len())?)
        .ok_or_else(|| anyhow::anyhow!("team collection error count overflow"))?;
    outcome.disposition = if outcome.errors == 0 && !report.rosters.blocked {
        Disposition::Complete
    } else {
        Disposition::Partial
    };
    Ok(outcome)
}

#[cfg(test)]
#[path = "teams_ingress_tests.rs"]
mod tests;
