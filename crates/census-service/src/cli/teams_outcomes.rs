use anyhow::Result;
use census_crawl::{AdapterReport, CollectionDisposition as Disposition};
use census_domain::UsJurisdiction;
use census_service::census::SourceAccessFailure;
use census_service::restate_services::TeamsSourceOutcome;
use serde::Serialize;
use std::fmt;

#[derive(Debug, thiserror::Error)]
pub(in crate::cli) struct TeamsCollectionError {
    pub(in crate::cli) failed: Vec<UsJurisdiction>,
    pub(in crate::cli) incomplete: Vec<UsJurisdiction>,
    pub(in crate::cli) disposition: Disposition,
}

impl fmt::Display for TeamsCollectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "team collection did not complete: failed [")?;
        codes(formatter, &self.failed)?;
        write!(formatter, "]; incomplete [")?;
        codes(formatter, &self.incomplete)?;
        write!(formatter, "]; outcome={:?}", self.disposition)
    }
}

fn codes(formatter: &mut fmt::Formatter<'_>, states: &[UsJurisdiction]) -> fmt::Result {
    for (index, jurisdiction) in states.iter().enumerate() {
        if index != 0 {
            write!(formatter, ", ")?;
        }
        write!(formatter, "{}", jurisdiction.code())?;
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct TeamsJurisdictionOutcome {
    jurisdiction: UsJurisdiction,
    report: AdapterReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure: Option<TeamsSourceOutcome>,
}

#[derive(Serialize)]
struct TeamsReport<'a> {
    disposition: Disposition,
    jurisdictions: &'a [TeamsJurisdictionOutcome],
}

#[derive(Debug, Default)]
pub(in crate::cli) struct TeamsOutcomes {
    failed: Vec<UsJurisdiction>,
    incomplete: Vec<UsJurisdiction>,
    jurisdictions: Vec<TeamsJurisdictionOutcome>,
}

impl TeamsOutcomes {
    pub(in crate::cli) fn record_report(
        &mut self,
        jurisdiction: UsJurisdiction,
        mut report: AdapterReport,
    ) -> String {
        if !report.disposition.is_complete() || report.errors != 0 || !report.unfinished.is_empty()
        {
            self.incomplete.push(jurisdiction);
            if report.disposition.is_complete() {
                report.disposition = Disposition::Partial;
            }
        }
        let line = format!(
            "{}\tteams={}\toutcome={:?}\terrors={}\tunfinished={}",
            jurisdiction.code(),
            report.rows,
            report.disposition,
            report.errors,
            report.unfinished.len()
        );
        self.jurisdictions.push(TeamsJurisdictionOutcome {
            jurisdiction,
            report,
            failure: None,
        });
        line
    }

    pub(in crate::cli) fn record_failure(
        &mut self,
        jurisdiction: UsJurisdiction,
        error: &SourceAccessFailure,
    ) -> String {
        let message = error.to_string();
        let failure = if error.retryable() {
            TeamsSourceOutcome::Interrupted {
                attempts: Some(1),
                message,
                progress: Vec::new(),
            }
        } else {
            TeamsSourceOutcome::Terminal {
                message,
                progress: Vec::new(),
            }
        };
        self.source_failure(jurisdiction, "milesplit", failure)
    }

    pub(in crate::cli) fn record_interruption(
        &mut self,
        jurisdiction: UsJurisdiction,
        error: &anyhow::Error,
    ) -> String {
        self.source_failure(
            jurisdiction,
            "jurisdiction",
            TeamsSourceOutcome::Interrupted {
                attempts: None,
                message: format!("{error:#}"),
                progress: Vec::new(),
            },
        )
    }

    fn source_failure(
        &mut self,
        jurisdiction: UsJurisdiction,
        source: &str,
        failure: TeamsSourceOutcome,
    ) -> String {
        let mut report = AdapterReport::new(source, "teams");
        let uncertain = matches!(
            &failure,
            TeamsSourceOutcome::Interrupted { attempts: None, .. }
        );
        let disposition = if uncertain {
            Disposition::Partial
        } else {
            Disposition::Failed
        };
        report.disposition = disposition;
        report.errors = 1;
        if uncertain {
            self.incomplete.push(jurisdiction);
        } else {
            self.failed.push(jurisdiction);
        }
        self.jurisdictions.push(TeamsJurisdictionOutcome {
            jurisdiction,
            report,
            failure: Some(failure),
        });
        format!(
            "{}\tteams=error\toutcome={disposition:?}",
            jurisdiction.code()
        )
    }
    pub(in crate::cli) fn disposition(&self) -> Disposition {
        if self.failed.is_empty() && self.incomplete.is_empty() {
            Disposition::Complete
        } else if self.failed.len() == self.jurisdictions.len() {
            Disposition::Failed
        } else {
            Disposition::Partial
        }
    }

    pub(in crate::cli) fn json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(&TeamsReport {
            disposition: self.disposition(),
            jurisdictions: &self.jurisdictions,
        })?)
    }

    pub(in crate::cli) fn finish(self) -> Result<()> {
        let disposition = self.disposition();
        if disposition.is_complete() {
            return Ok(());
        }
        Err(TeamsCollectionError {
            failed: self.failed,
            incomplete: self.incomplete,
            disposition,
        }
        .into())
    }
}
