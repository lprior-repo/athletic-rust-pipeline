//! One team page: its roster, dated by the season the page's own control states.
//!
//! A roster publishes no marks, so it contributes no performances; it is the head-count route a
//! list's top-N cannot produce.

use super::state::{Absorb, AthleteFacts, RosterContext, TeamFacts};
use crate::tfrrs::parse::{sport_from_route, ParsedRoster, RosterAthlete, YearToken};
use census_domain::model::{Gender, GradYear, ObservedGrade, SchoolId, SchoolYear, Sport};

/// What every athlete of one roster page is dated and titled by.
#[derive(Debug, Clone, Copy)]
struct PageSeason {
    sport: Sport,
    gender: Gender,
    school_year: SchoolYear,
}

impl<'a> Absorb<'a> {
    /// Absorb one team page's roster.
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
        // The roster's own rows carry no team link, so the page's team is minted from the season it
        // states; its athletes are minted from the roster table below.
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
            self.absorb_roster_row(context, &school, season, athlete, row_index);
        }
    }

    /// The season the page's own control states, with the route's sport filled in where the control
    /// is silent and the route is unambiguous (`xc`). A `tf` route cannot say indoor from outdoor,
    /// so it never guesses; a page that states neither is counted and its rows are skipped.
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

    /// Absorb one roster row: the athlete it names, with the grade its `YEAR` cell states.
    fn absorb_roster_row(
        &mut self,
        context: &RosterContext<'_>,
        school: &SchoolId,
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
        self.athlete_for(
            context.page,
            &AthleteFacts {
                school,
                name,
                grad_year: GradYear::of(grade, season.school_year),
                gender: season.gender,
                sport: season.sport,
                tfrrs_id: athlete.id,
                url: None,
                source_key: format!("{}:{}:roster:{row_index}", context.team.slug, season.school_year.get()),
                observed_grade: Some(ObservedGrade {
                    grade,
                    school_year: season.school_year,
                    source: context.page.source.clone(),
                }),
            },
        );
        self.stats.roster_rows_absorbed = self.stats.roster_rows_absorbed.saturating_add(1);
    }
}
