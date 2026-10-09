use super::{budget::ReadBudget, limits, ReadCounts, RowIssue};
use crate::CollectionDisposition;
use census_domain::school_directory::{DirectoryError, SchoolDirectoryEntry};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReadOutcome {
    entries: Vec<SchoolDirectoryEntry>,
    skipped: Vec<RowIssue>,
    notes: Vec<RowIssue>,
    retained_bytes: usize,
    stopped: Option<(usize, DirectoryError)>,
    drained: bool,
    budget: ReadBudget,
}

impl ReadOutcome {
    pub fn new() -> Self {
        Self::default()
    }
    pub(super) fn with_budget(budget: ReadBudget) -> Self {
        Self {
            budget,
            ..Self::default()
        }
    }
    pub(super) fn source_row_limit(&self) -> usize {
        self.budget.source_rows()
    }
    pub fn frontier_complete(&self) -> bool {
        self.drained && self.stopped.is_none()
    }
    pub fn entries(&self) -> &[SchoolDirectoryEntry] {
        &self.entries
    }
    pub fn skipped(&self) -> &[RowIssue] {
        &self.skipped
    }
    pub fn notes(&self) -> &[RowIssue] {
        &self.notes
    }
    pub fn into_entries(self) -> Vec<SchoolDirectoryEntry> {
        self.entries
    }
    pub fn unfinished(&self) -> Option<(usize, &DirectoryError)> {
        self.stopped.as_ref().map(|(line, error)| (*line, error))
    }
    pub fn stop(&mut self, line: usize, error: DirectoryError) {
        if self.stopped.is_none() {
            self.stopped = Some((line, error));
        }
        self.drained = false;
    }
    pub fn finish(&mut self) {
        self.drained = self.stopped.is_none();
    }
    pub fn disposition(&self) -> CollectionDisposition {
        if self.stopped.is_some() || !self.skipped.is_empty() || !self.notes.is_empty() {
            CollectionDisposition::Partial
        } else if self.drained {
            CollectionDisposition::Complete
        } else {
            CollectionDisposition::Unknown
        }
    }
    pub fn push(&mut self, entry: SchoolDirectoryEntry) -> Result<(), DirectoryError> {
        let retained = limits::admitted(self.retained_bytes, &entry, self.budget)?;
        limits::reserve(&mut self.entries, 1, self.budget)?;
        self.entries.push(entry);
        self.retained_bytes = retained;
        Ok(())
    }
    pub fn skip(
        &mut self,
        line: usize,
        field: &'static str,
        detail: impl AsRef<str>,
    ) -> Result<(), DirectoryError> {
        let issue = RowIssue::new(line, field, detail)?;
        let retained = limits::admitted(self.retained_bytes, &issue, self.budget)?;
        limits::reserve(&mut self.skipped, 1, self.budget)?;
        self.skipped.push(issue);
        self.retained_bytes = retained;
        Ok(())
    }
    pub fn note(
        &mut self,
        line: usize,
        field: &'static str,
        detail: impl AsRef<str>,
    ) -> Result<(), DirectoryError> {
        let issue = RowIssue::new(line, field, detail)?;
        let retained = limits::admitted(self.retained_bytes, &issue, self.budget)?;
        limits::reserve(&mut self.notes, 1, self.budget)?;
        self.notes.push(issue);
        self.retained_bytes = retained;
        Ok(())
    }
    pub fn counts(&self) -> ReadCounts {
        ReadCounts {
            entries: self.entries.len(),
            skipped: self.skipped.len(),
            notes: self.notes.len(),
        }
    }
    pub fn absorb(&mut self, other: ReadOutcome) -> Result<(), DirectoryError> {
        let retained = limits::add(self.retained_bytes, other.retained_bytes)?;
        limits::check(
            "directory retained bytes",
            retained,
            self.budget.retained_bytes(),
        )?;
        limits::reserve(&mut self.entries, other.entries.len(), self.budget)?;
        limits::reserve(&mut self.skipped, other.skipped.len(), self.budget)?;
        limits::reserve(&mut self.notes, other.notes.len(), self.budget)?;
        self.entries.extend(other.entries);
        self.skipped.extend(other.skipped);
        self.notes.extend(other.notes);
        self.retained_bytes = retained;
        self.drained = self.drained && other.drained;
        if self.stopped.is_none() {
            self.stopped = other.stopped;
        }
        Ok(())
    }
}
