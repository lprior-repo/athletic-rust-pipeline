use super::{Absorb, ListContext};
use crate::tfrrs::map::row::mark_with_adjustment;
use crate::tfrrs::parse::{
    ParsedAthlete, ParsedMeet, ParsedRow, ParsedSection, ParsedTeam, PublishedDate,
};
use census_domain::model::{Adjustment, Gender, Mark, Sport};

pub(super) struct RowFacts<'r> {
    pub(super) team: &'r ParsedTeam,
    pub(super) athlete: &'r ParsedAthlete,
    pub(super) name: &'r str,
    pub(super) date: &'r PublishedDate,
    pub(super) meet: &'r ParsedMeet,
    pub(super) mark: Mark,
    pub(super) adjustment: Option<Adjustment>,
    pub(super) sport: Sport,
    pub(super) ordinal: usize,
}

impl<'a> Absorb<'a> {
    pub(super) fn count_relay(&mut self, row: &ParsedRow) -> bool {
        if row.athlete.is_some() || row.relay_members.is_empty() {
            return false;
        }
        self.stats.rows_relay = self.stats.rows_relay.saturating_add(1);
        let members = u64::try_from(row.relay_members.len()).map_or(u64::MAX, |value| value);
        self.stats.relay_members = self.stats.relay_members.saturating_add(members);
        true
    }

    pub(super) fn row_facts<'r>(
        &mut self,
        context: &ListContext<'_>,
        row: &'r ParsedRow,
        ordinal: usize,
    ) -> Option<RowFacts<'r>> {
        let team = self.require_team(row)?;
        let sport = self.require_sport(context)?;
        let (athlete, name) = self.require_athlete(row)?;
        let date = self.require_date(row)?;
        let meet = self.require_meet(row)?;
        let (mark, adjustment) = self.require_mark(row)?;
        self.count_conversion(row, &mark);
        Some(RowFacts {
            team,
            athlete,
            name,
            date,
            meet,
            mark,
            adjustment,
            sport,
            ordinal,
        })
    }

    fn require_team<'r>(&mut self, row: &'r ParsedRow) -> Option<&'r ParsedTeam> {
        row.team.as_ref().map_or_else(
            || {
                self.stats.rows_without_team = self.stats.rows_without_team.saturating_add(1);
                None
            },
            Some,
        )
    }

    fn require_sport(&mut self, context: &ListContext<'_>) -> Option<Sport> {
        context
            .list
            .season
            .and_then(|season| season.sport)
            .filter(|sport| *sport != Sport::Unknown)
            .map_or_else(
                || {
                    self.stats.rows_without_season =
                        self.stats.rows_without_season.saturating_add(1);
                    None
                },
                Some,
            )
    }

    fn require_athlete<'r>(&mut self, row: &'r ParsedRow) -> Option<(&'r ParsedAthlete, &'r str)> {
        let athlete = row.athlete.as_ref()?;
        let name = athlete.full_name()?;
        Some((athlete, name))
    }

    fn require_date<'r>(&mut self, row: &'r ParsedRow) -> Option<&'r PublishedDate> {
        row.date.as_ref().map_or_else(
            || {
                self.stats.rows_without_date = self.stats.rows_without_date.saturating_add(1);
                None
            },
            Some,
        )
    }

    fn require_meet<'r>(&mut self, row: &'r ParsedRow) -> Option<&'r ParsedMeet> {
        row.meet.as_ref().map_or_else(
            || {
                self.stats.rows_without_meet = self.stats.rows_without_meet.saturating_add(1);
                None
            },
            Some,
        )
    }

    fn require_mark(&mut self, row: &ParsedRow) -> Option<(Mark, Option<Adjustment>)> {
        let result = row.mark.as_ref().and_then(|mark| {
            mark_with_adjustment(mark, row.conv_metres, row.converted_note.as_deref())
        });
        if result.is_none() {
            self.stats.rows_without_mark = self.stats.rows_without_mark.saturating_add(1);
        }
        result
    }

    fn count_conversion(&mut self, row: &ParsedRow, mark: &Mark) {
        if matches!(mark, Mark::Raw(_)) {
            self.stats.marks_unconverted = self.stats.marks_unconverted.saturating_add(1);
        }
        if row.converted_note.is_some() {
            self.stats.marks_converted = self.stats.marks_converted.saturating_add(1);
        }
    }

    pub(super) fn gender_of(&mut self, team: &ParsedTeam, section: &ParsedSection) -> Gender {
        match team.gender.or(section.gender) {
            Some(gender) => gender,
            None => {
                self.stats.genders_unknown = self.stats.genders_unknown.saturating_add(1);
                Gender::Unknown
            }
        }
    }
}
