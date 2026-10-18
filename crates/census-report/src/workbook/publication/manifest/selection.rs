use super::{invariant, Selection};
use crate::report::{ReportResult, Scope};
use crate::workbook::Options;

impl Selection {
    pub(super) fn of(options: &Options) -> Self {
        let mut jurisdictions: Vec<String> = options
            .jurisdictions
            .iter()
            .map(|jurisdiction| jurisdiction.code().to_string())
            .collect();
        jurisdictions.sort();
        jurisdictions.dedup();
        Self {
            scope: options.scope.as_str().to_string(),
            grad_year: options.grad_year,
            limit: options.limit,
            school_year: Some(options.school_year),
            jurisdictions,
        }
    }

    pub(super) fn options(&self) -> ReportResult<Options> {
        let scope = match self.scope.as_str() {
            "core" => Scope::Core,
            "all_sources" => Scope::AllSources,
            _ => return Err(invariant("invalid publication scope".to_string())),
        };
        let school_year = self.school_year.ok_or_else(|| {
            invariant("the publication manifest carries no contact school year".to_string())
        })?;
        let jurisdictions = self
            .jurisdictions
            .iter()
            .map(|code| {
                census_domain::UsJurisdiction::from_code(code).ok_or_else(|| {
                    invariant(format!(
                        "the publication manifest carries an unknown jurisdiction {code:?}"
                    ))
                })
            })
            .collect::<ReportResult<Vec<_>>>()?;
        Ok(Options {
            scope,
            grad_year: self.grad_year,
            limit: self.limit,
            school_year,
            out: None,
            jurisdictions,
        })
    }
}
