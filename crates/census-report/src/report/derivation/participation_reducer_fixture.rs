use super::Derivation;
use crate::bests::Options;
use crate::export::ExportDataset;
use crate::report::ReportResult;

impl<'a> Derivation<'a> {
    pub(crate) fn with_participation_reducer_fixture<T>(
        dataset: &'a ExportDataset,
        options: &Options,
        reduce: impl FnOnce(&Self) -> ReportResult<T>,
    ) -> ReportResult<T> {
        let derivation = Self::historical(
            dataset,
            options.scope,
            options.grad_year,
            &options.jurisdictions,
        );
        reduce(&derivation)
    }
}
