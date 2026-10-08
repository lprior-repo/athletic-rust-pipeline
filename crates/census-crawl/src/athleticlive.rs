use crate::coach_contacts::artifact::{ContactCsv, MAX_ROWS};
use crate::net::cache::CacheMeta;
use crate::{AdapterContext, AdapterReport, CollectionDisposition, CrawlError, CrawlResult};
use census_domain::UsJurisdiction;
use census_store::Table;

mod capture;
mod csv_capture;
mod docs;
mod map;
mod map_rows;
mod meets;
mod parse;
mod results;
mod standings;
mod wire;

pub use docs::parse_event_document;
pub use meets::build_meets;
pub use parse::{implausible_year, infer_level, parse_meets_csv, MeetRow};
pub use results::{
    collect as collect_results, collect_manifest, ManifestOptions, ResultOptions, StandingsCapture,
};
pub use wire::event_doc_url;

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub input: Option<String>,
    pub limit: Option<usize>,
    pub refresh: bool,
    pub input_metadata: Option<CacheMeta>,
    pub states: Vec<UsJurisdiction>,
    pub school_names: Vec<String>,
}

impl Options {
    pub fn for_input(input: impl Into<String>, metadata: CacheMeta) -> Self {
        Self {
            input: Some(input.into()),
            input_metadata: Some(metadata),
            ..Default::default()
        }
    }
}

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let input = options
        .input
        .as_deref()
        .ok_or_else(|| CrawlError::Invariant {
            detail: "athleticlive adapter requires a published meet CSV input".to_string(),
        })?;
    let captured = csv_capture::open(ctx, input, options.input_metadata.as_ref())?;
    collect_csv(ctx, options, input, captured)
}

fn collect_csv(
    ctx: &AdapterContext<'_>,
    options: &Options,
    input: &str,
    captured: csv_capture::FrozenCsv,
) -> CrawlResult<AdapterReport> {
    let mut csv = captured.csv;
    let mut report = AdapterReport::new("athleticlive", "meets");
    let header = match csv.record(0) {
        Ok(Some(record)) => record,
        Ok(None) => {
            fail(&mut report, input, 0, "CSV has no published header")?;
            return Ok(report);
        }
        Err(error) => {
            fail(&mut report, input, 0, &error.to_string())?;
            return Ok(report);
        }
    };
    if let Err(error) = parse::require_columns(&header) {
        fail(&mut report, input, 0, &error.to_string())?;
        return Ok(report);
    }
    let mut run = CsvRun {
        ctx,
        options,
        input,
        csv,
        header,
        capture: captured.capture,
        report,
        seen: 0,
        selected: 0,
    };
    let end = MAX_ROWS.checked_add(1).ok_or_else(counter_error)?;
    (1..=end)
        .find_map(|index| run.read(index).transpose())
        .transpose()?;
    Ok(run.report)
}

struct CsvRun<'a, 'ctx> {
    ctx: &'a AdapterContext<'ctx>,
    options: &'a Options,
    input: &'a str,
    csv: ContactCsv,
    header: csv::StringRecord,
    capture: csv_capture::CsvProvenance,
    report: AdapterReport,
    seen: usize,
    selected: usize,
}

impl CsvRun<'_, '_> {
    fn read(&mut self, index: usize) -> CrawlResult<Option<()>> {
        let record = match self.csv.record(index) {
            Ok(Some(record)) => record,
            Ok(None) => {
                if self.seen == 0 {
                    fail(
                        &mut self.report,
                        self.input,
                        1,
                        "CSV has no published meet rows",
                    )?;
                } else {
                    self.report.finish_frontier();
                }
                return Ok(Some(()));
            }
            Err(error) => {
                fail(&mut self.report, self.input, index, &error.to_string())?;
                return Ok(if self.csv.can_continue() {
                    None
                } else {
                    Some(())
                });
            }
        };
        if index > MAX_ROWS {
            fail(
                &mut self.report,
                self.input,
                index,
                "CSV row capacity reached; remaining input is owed",
            )?;
            return Ok(Some(()));
        }
        self.seen = index;
        self.process(record, index)?;
        Ok(None)
    }

    fn process(&mut self, record: csv::StringRecord, index: usize) -> CrawlResult<()> {
        if record.iter().any(|field| field.len() > 4096) {
            return fail(
                &mut self.report,
                self.input,
                index,
                "CSV field exceeds 4096 bytes",
            );
        }
        let row = match parse::meet_row(&self.header, &record, index) {
            Ok(row) => row,
            Err(error) => return fail(&mut self.report, self.input, index, &error.to_string()),
        };
        if !self.options.states.is_empty() && !self.options.states.contains(&row.state_code) {
            return Ok(());
        }
        self.selected = self.selected.checked_add(1).ok_or_else(counter_error)?;
        if self
            .options
            .limit
            .is_some_and(|limit| self.selected > limit)
        {
            return owe(&mut self.report, self.input, index);
        }
        if implausible_year(&row.start) {
            return fail(
                &mut self.report,
                self.input,
                index,
                "published meet year lies outside admitted range",
            );
        }
        let Some(date) = crate::context::published_performance_date(&row.start) else {
            return fail(
                &mut self.report,
                self.input,
                index,
                "CSV has no admitted published meet date",
            );
        };
        if date > self.ctx.performance_as_of {
            return owe(&mut self.report, self.input, index);
        }
        self.retain(&row)
    }

    fn retain(&mut self, row: &MeetRow) -> CrawlResult<()> {
        let mut meet = meets::project(row, &self.capture.acquired_on, "athleticlive_meets_csv");
        self.capture.bind(&mut meet);
        if self
            .ctx
            .append_row_once("athleticlive_meets_csv_projection_v1", Table::Meets, &meet)?
        {
            self.report.rows = self.report.rows.checked_add(1).ok_or_else(counter_error)?;
        }
        Ok(())
    }
}

fn owe(report: &mut AdapterReport, input: &str, index: usize) -> CrawlResult<()> {
    report
        .unfinished
        .try_reserve(1)
        .map_err(|_| CrawlError::Resource {
            resource: "LIVE CSV unfinished rows",
            requested: 1,
            limit: MAX_ROWS,
        })?;
    report.unfinished.push(format!("{input}#record={index}"));
    report.disposition = CollectionDisposition::Partial;
    Ok(())
}

fn fail(report: &mut AdapterReport, input: &str, index: usize, detail: &str) -> CrawlResult<()> {
    report.errors = report.errors.checked_add(1).ok_or_else(counter_error)?;
    owe(report, input, index)?;
    if report.errors <= 5 {
        report.note(detail.chars().take(4096).collect::<String>());
    }
    Ok(())
}

fn counter_error() -> CrawlError {
    CrawlError::Arithmetic {
        detail: "LIVE CSV accounting overflow".to_string(),
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod csv_capture_tests;
