use crate::bests::SharedSelection;
use crate::report::{Census, Derivation};

pub(in crate::workbook) struct RunFacts<'a> {
    pub(in crate::workbook) population: &'a Derivation<'a>,
    pub(in crate::workbook) recruiting: &'a Derivation<'a>,
    pub(in crate::workbook) core: &'a Census,
    pub(in crate::workbook) all_sources: &'a Census,
    pub(in crate::workbook) bests: &'a [SharedSelection],
    pub(in crate::workbook) school_year: census_domain::model::SchoolYear,
    pub(in crate::workbook) admission: &'a crate::report::EventAdmission<'a>,
}
