use super::artifact::{BoundedHashWriter, ContactCsv, MAX_RECORD_BYTES, MAX_ROWS};
use super::entities::row_entities;
use super::wire::CoachContactRow;
use crate::{AdapterReport, CollectionDisposition, CrawlError, CrawlResult};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use serde::Serialize;
use std::path::Path;

const PHASE: &str = "coach_contacts_csv_v2";
enum Progress {
    Read,
    Finished,
}

pub fn import_csv(
    store: &Store,
    path: &Path,
    default_observed_on: &str,
) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new("coach_contacts_csv", "coaches");
    let mut csv = ContactCsv::open(path, 0).map_err(|error| schema(path, error.to_string()))?;
    let header = match csv
        .record(0)
        .map_err(|error| schema(path, error.to_string()))?
    {
        Some(record) => record,
        None => {
            report.unfinished.push(path.display().to_string());
            return Ok(report);
        }
    };
    if !header.iter().any(|field| field == "school") || !header.iter().any(|field| field == "state")
    {
        fail(&mut report, path, 0, "CSV header omits school/state")?;
        return Ok(report);
    }
    let mut run = Run {
        store,
        path,
        csv,
        header,
        default_observed_on,
        report,
        seen: 0,
    };
    let end = MAX_ROWS.checked_add(1).ok_or_else(counter_error)?;
    (1..=end)
        .find_map(|index| match run.read(index) {
            Ok(Progress::Finished) => Some(Ok(())),
            Ok(Progress::Read) => None,
            Err(error) => Some(Err(error)),
        })
        .transpose()?;
    Ok(run.report)
}

struct Run<'a> {
    store: &'a Store,
    path: &'a Path,
    csv: ContactCsv,
    header: csv::StringRecord,
    default_observed_on: &'a str,
    report: AdapterReport,
    seen: usize,
}

impl Run<'_> {
    fn read(&mut self, index: usize) -> CrawlResult<Progress> {
        let fields = match self.csv.record(index) {
            Ok(Some(record)) => record,
            Ok(None) => {
                if self.seen > 0 {
                    self.report.finish_frontier();
                } else {
                    self.report.unfinished.push(self.path.display().to_string());
                }
                return Ok(Progress::Finished);
            }
            Err(error) => {
                fail(&mut self.report, self.path, index, &error.to_string())?;
                return Ok(if self.csv.can_continue() {
                    Progress::Read
                } else {
                    Progress::Finished
                });
            }
        };
        if index > MAX_ROWS {
            fail(
                &mut self.report,
                self.path,
                index,
                "CSV row capacity reached; remaining input is owed",
            )?;
            return Ok(Progress::Finished);
        }
        self.seen = index;
        if fields.iter().any(|value| value.len() > 4096) {
            fail(
                &mut self.report,
                self.path,
                index,
                "CSV field exceeds 4096 bytes",
            )?;
            return Ok(Progress::Read);
        }
        let parsed = fields
            .deserialize::<CoachContactRow>(Some(&self.header))
            .map_err(|error| schema(self.path, error.to_string()))
            .and_then(|row| contact_row(row, self.path));
        match parsed {
            Ok((row, state)) => self.project(&row, state, index)?,
            Err(error) => fail(&mut self.report, self.path, index, &error.to_string())?,
        }
        Ok(Progress::Read)
    }

    fn project(
        &mut self,
        row: &CoachContactRow,
        state: UsJurisdiction,
        index: usize,
    ) -> CrawlResult<()> {
        let entities = match row_entities(row, state, self.default_observed_on) {
            Ok(entities) => entities,
            Err(error) => {
                fail(&mut self.report, self.path, index, &error.to_string())?;
                return Ok(());
            }
        };
        write_row(self.store, Table::Schools, &entities.school)?;
        entities.coaches.iter().try_for_each(|coach| {
            if write_row(self.store, Table::Coaches, coach)? {
                self.report.rows = self.report.rows.checked_add(1).ok_or_else(counter_error)?;
                if coach.has_published_email() {
                    self.report.with_email = self
                        .report
                        .with_email
                        .checked_add(1)
                        .ok_or_else(counter_error)?;
                }
            }
            Ok::<_, CrawlError>(())
        })?;
        Ok(())
    }
}

fn contact_row(
    row: CoachContactRow,
    path: &Path,
) -> CrawlResult<(CoachContactRow, UsJurisdiction)> {
    if row.school.trim().is_empty() {
        return Err(schema(path, "row has no school".to_string()));
    }
    let state = UsJurisdiction::from_code(row.state.trim())
        .ok_or_else(|| schema(path, format!("invalid jurisdiction {:?}", row.state)))?;
    Ok((row, state))
}

fn write_row<T: Serialize>(store: &Store, table: Table, row: &T) -> CrawlResult<bool> {
    let mut writer = BoundedHashWriter::new(
        std::io::sink(),
        u64::try_from(MAX_RECORD_BYTES).map_err(|_| counter_error())?,
    );
    serde_json::to_writer(&mut writer, row).map_err(|source| CrawlError::Encode {
        table: table.file().to_string(),
        source,
    })?;
    let (_, digest) = writer.finish();
    let operation = format!("{PHASE}:{}:{digest}", table.file());
    let mut batch = store.write_batch();
    batch.record_many(table, std::slice::from_ref(row))?;
    Ok(batch.commit_once(&operation, &digest)?.written())
}

fn fail(report: &mut AdapterReport, path: &Path, index: usize, detail: &str) -> CrawlResult<()> {
    report.errors = report.errors.checked_add(1).ok_or_else(counter_error)?;
    report
        .unfinished
        .try_reserve(1)
        .map_err(|_| CrawlError::Resource {
            resource: "contact import unfinished rows",
            requested: 1,
            limit: MAX_ROWS,
        })?;
    report
        .unfinished
        .push(format!("{}#record={index}", path.display()));
    report.disposition = CollectionDisposition::Partial;
    if report.errors <= 5 {
        report.note(detail.chars().take(4096).collect::<String>());
    }
    Ok(())
}

fn schema(path: &Path, detail: String) -> CrawlError {
    CrawlError::Schema {
        url: path.display().to_string(),
        detail,
    }
}

fn counter_error() -> CrawlError {
    CrawlError::Arithmetic {
        detail: "contact import count overflow".to_string(),
    }
}
