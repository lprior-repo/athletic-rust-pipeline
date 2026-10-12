use crate::report::{artifact_corrupt, verifier_input, ReportError, ReportResult};
use std::fmt;
use std::path::{Path, PathBuf};

mod inflate;
mod names;
mod sheet;
mod xml;
mod zip;

#[cfg(test)]
mod tests;

pub(super) use sheet::Cells;

pub(super) const WORKBOOK: &str = "xl/workbook.xml";
pub(super) const WORKBOOK_RELS: &str = "xl/_rels/workbook.xml.rels";
pub(super) const SHARED_STRINGS: &str = "xl/sharedStrings.xml";

#[derive(Debug, Clone, Copy)]
pub(super) struct Scope<'a> {
    path: &'a Path,
    member: &'a str,
}

impl<'a> Scope<'a> {
    pub(super) const fn new(path: &'a Path, member: &'a str) -> Self {
        Self { path, member }
    }

    pub(super) const fn path(self) -> &'a Path {
        self.path
    }

    pub(super) fn corrupt(&self, reason: impl fmt::Display) -> ReportError {
        artifact_corrupt(self.path, self.message("", reason))
    }

    pub(super) fn outside(&self, reason: impl fmt::Display) -> ReportError {
        artifact_corrupt(
            self.path,
            self.message("outside the inline-string profile: ", reason),
        )
    }

    fn message(&self, prefix: &str, reason: impl fmt::Display) -> String {
        if self.member.is_empty() {
            format!("{prefix}{reason}")
        } else {
            format!("{prefix}member {}: {reason}", self.member)
        }
    }
}

pub(super) trait Sink {
    fn push(&mut self, bytes: &[u8]) -> ReportResult<()>;
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Profile {
    pub(super) container: u64,
    pub(super) member: u64,
    pub(super) inflated: u64,
    pub(super) token: usize,
    pub(super) cells: u64,
    pub(super) members: usize,
}

impl Profile {
    pub(super) const fn inline() -> Self {
        Self {
            container: 256 * 1024 * 1024,
            member: 64 * 1024 * 1024,
            inflated: 512 * 1024 * 1024,
            token: 1024 * 1024,
            cells: 32 * 1024 * 1024,
            members: 1_024,
        }
    }

    pub(super) fn validate(self) -> ReportResult<()> {
        if self.container == 0
            || self.member == 0
            || self.inflated == 0
            || self.token == 0
            || self.cells == 0
            || self.members == 0
        {
            return Err(verifier_input(
                "the inline-string readback profile declares a zero budget",
            ));
        }
        if self.member > self.container {
            return Err(verifier_input(
                "the inline-string readback profile budgets a member beyond its container",
            ));
        }
        Ok(())
    }
}

pub(super) struct Archive {
    path: PathBuf,
    container: zip::Container,
    profile: Profile,
}

impl Archive {
    pub(super) fn open(path: &Path, profile: Profile) -> ReportResult<Self> {
        profile.validate()?;
        let container = zip::Container::open(path, profile)?;
        Ok(Self {
            path: path.to_path_buf(),
            container,
            profile,
        })
    }

    pub(super) fn scope<'s>(&'s self, member: &'s str) -> Scope<'s> {
        Scope::new(&self.path, member)
    }

    pub(super) fn uses_shared_strings(&self) -> bool {
        self.container
            .entry(SHARED_STRINGS)
            .is_some_and(|entry| entry.uncompressed > 0)
    }

    pub(super) fn sheet_names(&self) -> ReportResult<Vec<String>> {
        Ok(names::worksheets(self)?
            .into_iter()
            .map(|sheet| sheet.name)
            .collect())
    }

    pub(super) fn read_sheet<C: Cells>(
        &self,
        name: &str,
        rows: usize,
        columns: usize,
        visit: &mut C,
    ) -> ReportResult<Option<usize>> {
        let sheets = names::worksheets(self)?;
        let Some(sheet) = sheets.iter().find(|sheet| sheet.name == name) else {
            return Ok(None);
        };
        let scope = Scope::new(&self.path, &sheet.member);
        let entry = self
            .container
            .entry(&sheet.member)
            .ok_or_else(|| scope.corrupt("the sheet member is missing from the container"))?;
        let walk = sheet::Walk::new(visit, scope, self.profile, name, rows, columns);
        let mut xml = xml::Xml::new(walk, scope, self.profile.token);
        self.container.read(entry, &scope, &mut xml)?;
        xml.finish()?.finish().map(Some)
    }

    pub(in crate::workbook::verify::xlsx) fn read_member<H: xml::Handler>(
        &self,
        member: &str,
        handler: H,
    ) -> ReportResult<H> {
        let scope = Scope::new(&self.path, member);
        let entry = self
            .container
            .entry(member)
            .ok_or_else(|| scope.corrupt("the member is missing from the container"))?;
        let mut xml = xml::Xml::new(handler, scope, self.profile.token);
        self.container.read(entry, &scope, &mut xml)?;
        xml.finish()
    }
}
