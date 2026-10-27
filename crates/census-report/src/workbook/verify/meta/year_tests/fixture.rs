use super::TestResult;
use crate::export::ExportDataset;
use crate::report::Scope;
use crate::workbook::Options;
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Evidence, Gender, GradYear, PublishedGraduation, SchoolYear,
    SourceIdentity, SourceNamespace, SourceRef,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::path::PathBuf;

pub(super) struct Fixture {
    pub(super) directory: tempfile::TempDir,
    pub(super) store: Store,
    pub(super) unverified_2027: String,
    pub(super) conflicting_2027: String,
    pub(super) unverified_2028: String,
    pub(super) conflicting_2028: String,
}

pub(super) fn options(year: Option<i16>) -> TestResult<Options> {
    Ok(Options {
        address_corpus: None,
        grad_year: year,
        out: None,
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: vec![UsJurisdiction::Wisconsin],
        school_year: SchoolYear::new(2026).ok_or("fixture season")?,
    })
}

pub(super) fn fixture() -> TestResult<Fixture> {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let (mut school, _) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Year Readback High",
        "year readback high",
        None,
    );
    school.evidence.push(evidence());
    store.append(Table::Schools, &school)?;
    athlete(&store, &school, "Admitted 2027", 2027, &[2027])?;
    athlete(&store, &school, "Admitted 2028 First", 2028, &[2028])?;
    athlete(&store, &school, "Admitted 2028 Second", 2028, &[2028])?;
    let unverified_2027 = athlete(&store, &school, "Unverified 2027", 2027, &[])?;
    let conflicting_2027 = athlete(&store, &school, "Conflicting 2027", 2027, &[2027, 2026])?;
    let unverified_2028 = athlete(&store, &school, "Unverified 2028", 2028, &[])?;
    let conflicting_2028 = athlete(&store, &school, "Conflicting 2028", 2028, &[2028, 2029])?;
    Ok(Fixture {
        directory,
        store,
        unverified_2027,
        conflicting_2027,
        unverified_2028,
        conflicting_2028,
    })
}

fn evidence() -> Evidence {
    Evidence::parsed(SourceRef::id("wiaa_results"), "2026-06-01")
}

fn athlete(
    store: &Store,
    school: &CanonicalSchool,
    name: &str,
    year: i16,
    published: &[i16],
) -> TestResult<String> {
    let mut athlete = CanonicalAthlete::new(
        &school.id,
        name,
        GradYear::new(year).ok_or("fixture cohort")?,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, name),
    )?;
    athlete.evidence.push(evidence());
    for year in published {
        athlete.published_graduations.push(PublishedGraduation {
            grad_year: GradYear::new(*year).ok_or("fixture published cohort")?,
            source: SourceRef::id(format!("wiaa_results_{year}")),
        });
    }
    store.append(Table::Athletes, &athlete)?;
    Ok(athlete.id.to_string())
}

impl Fixture {
    pub(super) fn publish(&self, year: Option<i16>) -> TestResult<(PathBuf, ExportDataset)> {
        let workbook = crate::workbook::build(&self.store, &options(year)?)?;
        let frozen = workbook
            .parent()
            .ok_or("fixture generation directory")?
            .join("frozen-input.json");
        Ok((workbook, ExportDataset::reopen_frozen(&frozen)?))
    }

    pub(super) fn review_ids(&self, year: Option<i16>) -> Vec<String> {
        match year {
            Some(2027) => vec![self.unverified_2027.clone(), self.conflicting_2027.clone()],
            Some(2028) => vec![self.unverified_2028.clone(), self.conflicting_2028.clone()],
            _ => vec![
                self.unverified_2027.clone(),
                self.conflicting_2027.clone(),
                self.unverified_2028.clone(),
                self.conflicting_2028.clone(),
            ],
        }
    }

    pub(super) fn conflict_ids(&self, year: Option<i16>) -> Vec<String> {
        match year {
            Some(2027) => vec![self.conflicting_2027.clone()],
            Some(2028) => vec![self.conflicting_2028.clone()],
            _ => vec![self.conflicting_2027.clone(), self.conflicting_2028.clone()],
        }
    }
}
