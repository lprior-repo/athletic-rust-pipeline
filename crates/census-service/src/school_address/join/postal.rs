use census_domain::model::CanonicalSchool;
use census_domain::school_directory::SourceLabel;
use census_store::StoreResult;

use super::support::{bump, row};
use super::Job;

fn has_street(school: &CanonicalSchool) -> bool {
    school
        .postal_addresses
        .iter()
        .any(|claim| claim.address().line1().is_some())
}

impl Job<'_> {
    pub(super) fn gap_reason(
        &self,
        school: &CanonicalSchool,
        state: census_domain::UsJurisdiction,
        member: Option<&str>,
    ) -> &'static str {
        let (name, normalized, aliases) = match member {
            Some(name) => (name, name, &[][..]),
            None => (
                school.name.as_str(),
                school.normalized_name.as_str(),
                school.aliases.as_slice(),
            ),
        };
        match self.index.corroboration_gap(
            name,
            normalized,
            school.city.as_deref(),
            state,
            aliases,
            &self.attested,
        ) {
            Some(reason) => reason,
            None => "no_match",
        }
    }

    pub(super) fn visit(&mut self, school: &CanonicalSchool) -> StoreResult<()> {
        bump(&mut self.counters.scanned)?;
        let gap = !has_street(school);
        self.postal_present = !gap;
        self.postal_provider = None;
        let outcomes_before = self.outcomes.len();
        self.visit_school(school)?;
        self.mark_gap_outcomes(outcomes_before, gap && !self.postal_present)?;
        if !gap {
            return Ok(());
        }
        bump(&mut self.counters.postal_gaps)?;
        if self.postal_present {
            return self.count_fill();
        }
        bump(&mut self.counters.postal_remaining)?;
        if self.outcomes.len() == outcomes_before {
            let mut outcome = row(school, school.state, "postal_missing");
            outcome.reason = Some("no_published_street".to_string());
            outcome.detail = Some("the matched directory record supplies no evidenced street; an identity or website link is not a postal fill".to_string());
            outcome.postal_gap = true;
            self.outcomes.push(outcome);
        }
        Ok(())
    }

    fn mark_gap_outcomes(&mut self, start: usize, gap: bool) -> StoreResult<()> {
        let rows =
            self.outcomes
                .get_mut(start..)
                .ok_or_else(|| census_store::StoreError::Invariant {
                    detail: "school-address outcome boundary exceeds retained rows".to_string(),
                })?;
        rows.iter_mut().for_each(|row| row.postal_gap = gap);
        Ok(())
    }

    fn count_fill(&mut self) -> StoreResult<()> {
        bump(&mut self.counters.postal_filled)?;
        match self.postal_provider {
            Some("nces-ccd") => bump(&mut self.counters.ccd_filled),
            Some("nces-pss") => bump(&mut self.counters.pss_filled),
            _ => Ok(()),
        }
    }

    pub(super) fn record_postal(&mut self, school: &CanonicalSchool) {
        if self.postal_present {
            return;
        }
        let Some(claim) = school
            .postal_addresses
            .iter()
            .find(|claim| claim.address().line1().is_some())
        else {
            return;
        };
        self.postal_present = true;
        self.postal_provider = match claim.source_label() {
            SourceLabel::Ccd => Some("nces-ccd"),
            SourceLabel::Pss => Some("nces-pss"),
            _ => None,
        };
    }
}
