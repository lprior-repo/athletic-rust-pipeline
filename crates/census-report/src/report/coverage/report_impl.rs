use super::{add, totals_text, CoverageReport, CoverageTotals, ReportError, ReportResult};

impl CoverageReport {
    pub fn publish_coverage_rows(
        store: &census_store::Store,
        evidence: &[census_domain::model::CoverageRow],
    ) -> ReportResult<usize> {
        super::population::publish_coverage_rows(store, evidence)
    }

    pub fn published_totals(&self) -> CoverageTotals {
        let mut totals = CoverageTotals::default();
        for row in &self.jurisdictions {
            add(&mut totals.schools, row.schools);
            add(&mut totals.athletes, row.athletes);
            add(&mut totals.coaches, row.coaches);
            add(&mut totals.meets, row.meets);
            add(&mut totals.performances, row.performances);
        }
        totals
    }

    pub fn reconcile(&self) -> ReportResult<()> {
        self.reconcile_lifecycle()?;
        self.reconcile_contact()?;
        self.reconcile_totals()
    }

    fn reconcile_lifecycle(&self) -> ReportResult<()> {
        let published = self.published_totals();
        if published != self.read {
            return Err(ReportError::Invariant {
                detail: format!(
                    "coverage reconciliation failed: read {} but the rows publish {}",
                    totals_text(&self.read),
                    totals_text(&published)
                ),
            });
        }

        if self.raw_observations < self.scope_filtered {
            return Err(ReportError::Invariant {
                detail: format!(
                    "lifecycle reconciliation failed: raw_observations {} < scope_filtered {}",
                    self.raw_observations, self.scope_filtered
                ),
            });
        }
        if self.scope_filtered < self.accepted_cohort {
            return Err(ReportError::Invariant {
                detail: format!(
                    "lifecycle reconciliation failed: scope_filtered {} < accepted_cohort {}",
                    self.scope_filtered, self.accepted_cohort
                ),
            });
        }
        Ok(())
    }

    fn reconcile_contact(&self) -> ReportResult<()> {
        let contact_total = self
            .contact
            .reached
            .saturating_add(self.contact.failed)
            .saturating_add(self.contact.blocked);
        if self.contact.attempted < contact_total {
            return Err(ReportError::Invariant {
                detail: format!(
                    "contact reconciliation failed: attempted {} < reached+failed+blocked {}",
                    self.contact.attempted, contact_total
                ),
            });
        }
        if self.contact.terminal() > self.contact.attempted {
            return Err(ReportError::Invariant {
                detail: format!(
                    "contact reconciliation failed: terminal {} > attempted {}",
                    self.contact.terminal(),
                    self.contact.attempted
                ),
            });
        }
        Ok(())
    }

    fn reconcile_totals(&self) -> ReportResult<()> {
        let declared: usize = self.state_counts.values().sum();
        if declared != self.jurisdictions.len() {
            return Err(ReportError::Invariant {
                detail: format!(
                    "coverage state reconciliation failed: {} declared states for {} jurisdiction rows",
                    declared,
                    self.jurisdictions.len()
                ),
            });
        }
        for row in &self.jurisdictions {
            let state = super::state_slug(row.state);
            if !self.state_counts.contains_key(state) {
                return Err(ReportError::Invariant {
                    detail: format!(
                        "coverage state reconciliation failed: row {} is absent from the declared states",
                        row.jurisdiction.code()
                    ),
                });
            }
            row.source_population.reconcile()?;
        }
        self.population_totals()?;
        self.source_population.reconcile()?;
        Ok(())
    }

    fn population_totals(&self) -> ReportResult<()> {
        let global = [
            self.source_population.registered,
            self.source_population.applicable,
            self.source_population.attempted,
            self.source_population.captured,
            self.source_population.parsed,
            self.source_population.accepted,
            self.source_population.withheld,
        ];
        let mut sums = [0usize; 7];
        let mut measured = [true; 7];
        for row in &self.jurisdictions {
            let stages = [
                row.source_population.registered,
                row.source_population.applicable,
                row.source_population.attempted,
                row.source_population.captured,
                row.source_population.parsed,
                row.source_population.accepted,
                row.source_population.withheld,
            ];
            for ((sum, measured), value) in sums.iter_mut().zip(measured.iter_mut()).zip(stages) {
                match value {
                    Some(value) => *sum = sum.saturating_add(value),
                    None => *measured = false,
                }
            }
        }
        for ((total, sum), (measured, stage)) in global
            .into_iter()
            .zip(sums.iter())
            .zip(measured.iter().zip(super::population::STAGES.iter()))
        {
            if let Some(total) = total {
                if *measured && *sum > total {
                    return Err(ReportError::Invariant {
                        detail: format!(
                            "source population reconciliation failed: rows sum {sum} > published {total} for stage {stage}"
                        ),
                    });
                }
            }
        }
        Ok(())
    }

    pub fn reconciliation_lines(&self) -> Vec<String> {
        let published = self.published_totals();
        let verdict = if published == self.read {
            "matches"
        } else {
            "MISMATCH"
        };
        let cohort = self
            .grad_year
            .map_or_else(|| "all".to_string(), |year| year.to_string());
        vec![
            format!(
                "coverage cohort: grad_year={cohort} off_cohort_athletes={}",
                self.off_cohort_athletes
            ),
            format!("coverage read: {}", totals_text(&self.read)),
            format!("coverage published: {}", totals_text(&published)),
            format!("coverage reconciliation: {verdict}"),
        ]
    }
}
