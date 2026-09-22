//! One performance-list page: one `CanonicalPerformance` per published mark.

use super::row::{grade_for, mark_of, source_key};
use super::state::{Absorb, AthleteFacts, ListContext, TeamFacts};
use crate::sources::tfrrs::parse::{
    parse_team_path, ParsedAthlete, ParsedList, ParsedMeet, ParsedRow, ParsedSection, ParsedTeam,
    PublishedDate,
};
use census_domain::model::{
    AthleteId, CanonicalPerformance, EventId, EventKind, Evidence, EvidenceMethod, Gender,
    GradYear, Grade, Mark, MeetId, ObservedGrade, SchoolId, SchoolYear, Sport, TeamId,
};

/// The entities one row mints, so the record below is one straight line of fields.
struct RowMints {
    team: TeamId,
    meet: MeetId,
    event: EventId,
    kind: EventKind,
    athlete: AthleteId,
}

/// The row's own facts, read once so that the mint below is one straight line of calls.
struct RowFacts<'r> {
    team: &'r ParsedTeam,
    athlete: &'r ParsedAthlete,
    name: &'r str,
    date: &'r PublishedDate,
    meet: &'r ParsedMeet,
    mark: Mark,
    sport: Sport,
}

impl<'a> Absorb<'a> {
    /// Absorb one performance-list page.
    pub(in crate::sources::tfrrs) fn absorb_list(
        &mut self,
        context: &ListContext<'_>,
        page: &ParsedList,
    ) {
        for section in &page.sections {
            self.stats.sections = self.stats.sections.saturating_add(1);
            for row in &section.rows {
                self.absorb_row(context, section, row);
            }
        }
    }

    /// Absorb one row of a performance list.
    fn absorb_row(&mut self, context: &ListContext<'_>, section: &ParsedSection, row: &ParsedRow) {
        self.stats.rows_seen = self.stats.rows_seen.saturating_add(1);
        if self.count_relay(row) {
            return;
        }
        let Some(facts) = self.row_facts(context, row) else {
            return;
        };
        let Some(grade) = grade_for(row, context.filter, &mut self.stats) else {
            self.stats.rows_without_grade = self.stats.rows_without_grade.saturating_add(1);
            return;
        };
        let school_year = facts.date.school_year();
        let gender = self.gender_of(facts.team, section);
        let Some(school) = self.school_for(context.page, &facts.team.name) else {
            return;
        };
        self.mint_row(
            context,
            section,
            row,
            &facts,
            (grade, school_year, gender),
            &school,
        );
        self.stats.rows_absorbed = self.stats.rows_absorbed.saturating_add(1);
    }

    /// Count a relay row's members and report whether the row is one. A relay prints surnames and no
    /// class year, so minting an athlete from it would invent an identity: the run counts instead.
    fn count_relay(&mut self, row: &ParsedRow) -> bool {
        if row.athlete.is_some() || row.relay_members.is_empty() {
            return false;
        }
        self.stats.rows_relay = self.stats.rows_relay.saturating_add(1);
        let members = u64::try_from(row.relay_members.len()).unwrap_or(u64::MAX);
        self.stats.relay_members = self.stats.relay_members.saturating_add(members);
        true
    }

    /// The row's own facts, in the order a mark needs them; each absence is counted and ends the row.
    fn row_facts<'r>(
        &mut self,
        context: &ListContext<'_>,
        row: &'r ParsedRow,
    ) -> Option<RowFacts<'r>> {
        let Some(team) = row.team.as_ref() else {
            self.stats.rows_without_team = self.stats.rows_without_team.saturating_add(1);
            return None;
        };
        // The list route states the sport; a path without it (a hand-built URL) states no season.
        let Some(sport) = context.list.season.and_then(|season| season.sport) else {
            self.stats.rows_without_season = self.stats.rows_without_season.saturating_add(1);
            return None;
        };
        let Some(athlete) = row.athlete.as_ref() else {
            self.stats.rows_without_athlete = self.stats.rows_without_athlete.saturating_add(1);
            return None;
        };
        let Some(name) = athlete.full_name() else {
            self.stats.rows_without_athlete = self.stats.rows_without_athlete.saturating_add(1);
            return None;
        };
        let Some(date) = row.date.as_ref() else {
            self.stats.rows_without_date = self.stats.rows_without_date.saturating_add(1);
            return None;
        };
        let Some(meet) = row.meet.as_ref() else {
            self.stats.rows_without_meet = self.stats.rows_without_meet.saturating_add(1);
            return None;
        };
        let Some(mark) = row
            .mark
            .as_ref()
            .and_then(|mark| mark_of(mark, row.conv_metres))
        else {
            self.stats.rows_without_mark = self.stats.rows_without_mark.saturating_add(1);
            return None;
        };
        if matches!(mark, Mark::Raw(_)) {
            self.stats.marks_unconverted = self.stats.marks_unconverted.saturating_add(1);
        }
        if row.converted_note.is_some() {
            self.stats.marks_converted = self.stats.marks_converted.saturating_add(1);
        }
        Some(RowFacts {
            team,
            athlete,
            name,
            date,
            meet,
            mark,
            sport,
        })
    }

    /// The gender a row's section states, through the team route's own side when the row links one.
    fn gender_of(&mut self, team: &ParsedTeam, section: &ParsedSection) -> Gender {
        match team.gender.or(section.gender) {
            Some(gender) => gender,
            None => {
                self.stats.genders_unknown = self.stats.genders_unknown.saturating_add(1);
                Gender::Unknown
            }
        }
    }

    /// Write the school, team, meet, event, athlete and performance one row implies.
    fn mint_row(
        &mut self,
        context: &ListContext<'_>,
        section: &ParsedSection,
        row: &ParsedRow,
        facts: &RowFacts<'_>,
        observed: (Grade, SchoolYear, Gender),
        school: &SchoolId,
    ) {
        let (grade, _, _) = observed;
        let mints = self.mint_entities(context, section, facts, observed, school);
        let source_key = source_key(facts.date, facts.meet, facts.athlete.id, section, row);
        let id = CanonicalPerformance::mint(
            &mints.athlete,
            &mints.meet,
            &mints.kind,
            &facts.date.iso,
            &source_key,
        );
        let performance = CanonicalPerformance {
            id: id.clone(),
            athlete: mints.athlete,
            team: mints.team,
            event: mints.event,
            meet: mints.meet,
            date: facts.date.iso.clone(),
            mark: facts.mark.clone(),
            wind_mps: row.wind,
            place: row.place,
            heat: None,
            round: None,
            timing: None,
            observed_grade: Some(grade),
            evidence: row_evidence(context, row),
            source_key,
        };
        self.accumulator
            .performances
            .entry(id.as_str().to_string())
            .or_insert(performance);
    }

    /// Mint the school-side entities a row implies: its team, its meet, its event and its athlete.
    fn mint_entities(
        &mut self,
        context: &ListContext<'_>,
        section: &ParsedSection,
        facts: &RowFacts<'_>,
        observed: (Grade, SchoolYear, Gender),
        school: &SchoolId,
    ) -> RowMints {
        let (grade, school_year, gender) = observed;
        let published_route = parse_team_path(&facts.team.path);
        let team = self.team_for(
            context.page,
            &TeamFacts {
                school,
                sport: facts.sport,
                gender,
                school_year,
                slug: published_route.as_ref().map(|path| path.slug.as_str()),
                path: Some(facts.team.path.as_str()),
            },
        );
        let meet_id = self.meet_for(context.page, facts.meet, facts.date);
        let (event, kind) = self.event_for(context.page, section, &meet_id, gender);
        let athlete_id = self.athlete_for(
            context.page,
            &AthleteFacts {
                school,
                name: facts.name,
                grad_year: GradYear::of(grade, school_year),
                gender,
                sport: facts.sport,
                tfrrs_id: facts.athlete.id,
                url: None,
                observed_grade: Some(ObservedGrade {
                    grade,
                    school_year,
                    source: context.page.source.clone(),
                }),
            },
        );
        RowMints {
            team,
            meet: meet_id,
            event,
            kind,
            athlete: athlete_id,
        }
    }
}

/// The evidence one row's performance carries: the page it was read from, plus the host's own
/// conversion note when the row published one.
fn row_evidence(context: &ListContext<'_>, row: &ParsedRow) -> Vec<Evidence> {
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
