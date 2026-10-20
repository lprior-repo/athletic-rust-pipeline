mod admission;
mod row_facts;
mod row_mint;
use super::row::{admit_date, grade_for};
use super::state::{Absorb, AthleteFacts, ListContext, TeamFacts};
use crate::tfrrs::parse::ParsedList;
use census_domain::model::{Evidence, EvidenceMethod};

impl<'a> Absorb<'a> {
    pub(in crate::tfrrs) fn absorb_list(
        &mut self,
        context: &ListContext<'_>,
        page: &ParsedList,
    ) -> crate::CrawlResult<()> {
        let mut outcome = Ok(());
        for section in &page.sections {
            self.stats.sections = self.stats.sections.saturating_add(1);
            for (ordinal, row) in section.rows.iter().enumerate() {
                let projected = self.absorb_row(context, section, row, ordinal);
                outcome = outcome.and(projected);
            }
        }
        outcome
    }

    fn absorb_row(
        &mut self,
        context: &ListContext<'_>,
        section: &crate::tfrrs::parse::ParsedSection,
        row: &crate::tfrrs::parse::ParsedRow,
        ordinal: usize,
    ) -> crate::CrawlResult<()> {
        self.stats.rows_seen = self.stats.rows_seen.saturating_add(1);
        if self.count_relay(row) {
            return Ok(());
        }
        if !admit_date(context.page, row)? {
            return Ok(());
        }
        let Some(facts) = self.row_facts(context, row, ordinal) else {
            return Ok(());
        };
        let Some(grade) = grade_for(row, context.filter, &mut self.stats) else {
            return Ok(());
        };
        let Some(school_year) = facts.date.school_year() else {
            self.stats.rows_without_season = self.stats.rows_without_season.saturating_add(1);
            return Ok(());
        };
        let gender = self.gender_of(facts.team, section);
        let Some(school) = self.school_for(context.page, &facts.team.name) else {
            return Ok(());
        };
        if self
            .mint_row(
                context,
                section,
                row,
                &facts,
                (grade, school_year, gender),
                &school,
            )?
            .is_some()
        {
            self.stats.rows_absorbed = self.stats.rows_absorbed.saturating_add(1);
        }
        Ok(())
    }
}

fn row_evidence(context: &ListContext<'_>, row: &crate::tfrrs::parse::ParsedRow) -> Vec<Evidence> {
    let mut evidence = vec![Evidence::parsed(
        context.page.source.clone(),
        context.page.observed_on,
    )];
    if let Some(note) = row.converted_note.as_deref() {
        evidence.push(Evidence {
            source: context.page.source.clone(),
            method: EvidenceMethod::Parsed,
            observed_on: context.page.observed_on.to_string(),
            note: Some(note.to_string()),
        });
    }
    evidence
}
