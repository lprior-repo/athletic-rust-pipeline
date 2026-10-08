use super::{Ctx, ResolvedRow};
use crate::athleticnet::map::{store_performance, PerformanceInput};
use crate::athleticnet::parse::{round_of, TfRow, XcRow};
use census_domain::model::{AthleteId, Gender, Sport};

impl Ctx<'_> {
    pub(super) fn store_tf_row(
        &mut self,
        row: &TfRow,
        resolved: &ResolvedRow<'_>,
        athlete_id: &AthleteId,
        gender: Gender,
    ) -> crate::CrawlResult<()> {
        let source_key = format!("athleticnet:{}-{}", self.target.athlete_id, row.id);
        let labels = [resolved.label.map_or("", core::convert::identity)];
        store_performance(
            self.accumulated,
            self.source,
            self.observed_on,
            PerformanceInput {
                athlete: athlete_id,
                source_athlete: self.source_athlete.clone(),
                school: &resolved.school,
                meet: &resolved.meet,
                kind: &resolved.kind,
                sport: Some(resolved.sport),
                gender,
                school_year: resolved.school_year,
                performance_as_of: self.performance_as_of,
                grade: resolved.grade,
                date: resolved.date.clone(),
                mark: resolved.mark.clone(),
                wind_mps: row.wind,
                place: row.place.as_deref(),
                round: round_of(row.round.as_deref()),
                timing: resolved.timing,
                division: row.division.clone(),
                source_key,
                labels: &labels,
            },
        )?;
        self.stats.rows_absorbed = self.stats.rows_absorbed.saturating_add(1);
        Ok(())
    }

    pub(super) fn store_xc_row(
        &mut self,
        row: &XcRow,
        resolved: &ResolvedRow<'_>,
        athlete_id: &AthleteId,
        gender: Gender,
    ) -> crate::CrawlResult<()> {
        let source_key = format!("athleticnet:{}-{}", self.target.athlete_id, row.id);
        let label = row.distance.map(|distance| format!("{distance}m"));
        let labels = [label.as_deref().map_or("", core::convert::identity)];
        store_performance(
            self.accumulated,
            self.source,
            self.observed_on,
            PerformanceInput {
                athlete: athlete_id,
                source_athlete: self.source_athlete.clone(),
                school: &resolved.school,
                meet: &resolved.meet,
                kind: &resolved.kind,
                sport: Some(Sport::CrossCountry),
                gender,
                school_year: resolved.school_year,
                performance_as_of: self.performance_as_of,
                grade: resolved.grade,
                date: resolved.date.clone(),
                mark: resolved.mark.clone(),
                wind_mps: None,
                place: row.place.as_deref(),
                round: None,
                timing: resolved.timing,
                division: row.division.clone(),
                source_key,
                labels: &labels,
            },
        )?;
        self.stats.rows_absorbed = self.stats.rows_absorbed.saturating_add(1);
        Ok(())
    }
}
