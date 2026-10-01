use super::entity::athlete_source;
use super::state::{Absorb, AthleteFacts, RosterContext, TeamFacts};
use crate::tfrrs::parse::{sport_from_route, ParsedRoster, RosterAthlete, YearToken};
use census_domain::model::{
    Gender, GradYear, Grade, ObservedGrade, SchoolId, SchoolYear, SourceAthleteObservation,
    SourceIdentity, Sport,
};

#[derive(Debug, Clone, Copy)]
struct PageSeason {
    sport: Sport,
    gender: Gender,
    school_year: SchoolYear,
}

impl<'a> Absorb<'a> {
    pub(in crate::tfrrs) fn absorb_roster(
        &mut self,
        context: &RosterContext<'_>,
        roster: &ParsedRoster,
    ) {
        self.stats.rosters_seen = self.stats.rosters_seen.saturating_add(1);
        let Some(season) = self.page_season(context, roster) else {
            return;
        };
        let Some(school_name) = roster.school.as_deref() else {
            return;
        };
        let Some(school) = self.school_for(context.page, school_name) else {
            return;
        };
        self.team_for(
            context.page,
            &TeamFacts {
                school: &school,
                sport: season.sport,
                gender: season.gender,
                school_year: season.school_year,
                slug: Some(context.team.slug.as_str()),
                path: None,
            },
        );
        for (row_index, athlete) in roster.athletes.iter().enumerate() {
            self.absorb_roster_row(context, &school, school_name, season, athlete, row_index);
        }
    }

    fn page_season(
        &mut self,
        context: &RosterContext<'_>,
        roster: &ParsedRoster,
    ) -> Option<PageSeason> {
        let stated = roster.season;
        let sport = stated
            .and_then(|season| season.sport)
            .or_else(|| sport_from_route(&context.team.route));
        let school_year = stated
            .map(|season| season.with_sport(sport))
            .and_then(|season| season.school_year());
        let (Some(sport), Some(school_year)) = (sport, school_year) else {
            self.stats.rosters_without_season = self.stats.rosters_without_season.saturating_add(1);
            return None;
        };
        Some(PageSeason {
            sport,
            gender: context.team.gender.unwrap_or(Gender::Unknown),
            school_year,
        })
    }

    fn admit_roster_athlete(
        &mut self,
        context: &RosterContext<'_>,
        school_name: &str,
        season: PageSeason,
        athlete: &RosterAthlete,
        row_index: usize,
        identity: (&str, Grade),
    ) -> Option<(GradYear, ObservedGrade, SourceIdentity)> {
        let (name, grade) = identity;
        let source_key = format!(
            "{}:{}:roster:{row_index}",
            context.team.slug,
            season.school_year.get(),
        );
        let observation = ObservedGrade {
            grade,
            school_year: season.school_year,
            source: context.page.source.clone(),
        };
        let source = athlete_source(athlete.id, &source_key);
        self.accumulator
            .unsupported
            .admit(observation, source, |source| {
                SourceAthleteObservation::new(
                    source.namespace,
                    source.id,
                    source_key,
                    name,
                    context.page.observed_on,
                )
                .with_school(Some(school_name.to_string()))
                .with_gender(season.gender)
            })
    }

    fn absorb_roster_row(
        &mut self,
        context: &RosterContext<'_>,
        school: &SchoolId,
        school_name: &str,
        season: PageSeason,
        athlete: &RosterAthlete,
        row_index: usize,
    ) {
        self.stats.roster_rows_seen = self.stats.roster_rows_seen.saturating_add(1);
        let Some(name) = athlete.full_name() else {
            self.stats.roster_rows_without_name =
                self.stats.roster_rows_without_name.saturating_add(1);
            return;
        };
        let Some(grade) = athlete.year.and_then(YearToken::grade) else {
            self.stats.roster_rows_without_year =
                self.stats.roster_rows_without_year.saturating_add(1);
            return;
        };
        let Some((grad_year, observation, source)) = self.admit_roster_athlete(
            context,
            school_name,
            season,
            athlete,
            row_index,
            (name, grade),
        ) else {
            return;
        };
        let _ = self.athlete_for(
            context.page,
            AthleteFacts {
                school,
                name,
                grad_year,
                gender: season.gender,
                sport: season.sport,
                source,
                observed_grade: Some(observation),
            },
        );
        self.stats.roster_rows_absorbed = self.stats.roster_rows_absorbed.saturating_add(1);
    }
}
