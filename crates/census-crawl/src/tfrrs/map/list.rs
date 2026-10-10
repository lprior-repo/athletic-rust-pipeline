mod admission;
use super::row::{admit_date, grade_for, mark_of, source_key};
use super::state::{Absorb, AthleteFacts, ListContext, TeamFacts};
use crate::tfrrs::parse::{
    parse_team_path, ParsedAthlete, ParsedList, ParsedMeet, ParsedRow, ParsedSection, ParsedTeam,
    PublishedDate,
};
use census_domain::model::{
    AthleteId, CanonicalPerformance, EventId, Evidence, EvidenceMethod, Gender, Grade, Mark,
    MeetId, SchoolId, SchoolYear, SourceIdentity, Sport, TeamId,
};

struct RowMints {
    team: TeamId,
    meet: MeetId,
    event: EventId,
    athlete: AthleteId,
    source_athlete: SourceIdentity,
}

struct RowFacts<'r> {
    team: &'r ParsedTeam,
    athlete: &'r ParsedAthlete,
    name: &'r str,
    date: &'r PublishedDate,
    meet: &'r ParsedMeet,
    mark: Mark,
    sport: Sport,
    ordinal: usize,
}

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
        section: &ParsedSection,
        row: &ParsedRow,
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
            self.stats.rows_without_grade = self.stats.rows_without_grade.saturating_add(1);
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

    fn count_relay(&mut self, row: &ParsedRow) -> bool {
        if row.athlete.is_some() || row.relay_members.is_empty() {
            return false;
        }
        self.stats.rows_relay = self.stats.rows_relay.saturating_add(1);
        let members = u64::try_from(row.relay_members.len()).map_or(u64::MAX, |value| value);
        self.stats.relay_members = self.stats.relay_members.saturating_add(members);
        true
    }

    fn row_facts<'r>(
        &mut self,
        context: &ListContext<'_>,
        row: &'r ParsedRow,
        ordinal: usize,
    ) -> Option<RowFacts<'r>> {
        let Some(team) = row.team.as_ref() else {
            self.stats.rows_without_team = self.stats.rows_without_team.saturating_add(1);
            return None;
        };
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
        let Some(mark) = row.mark.as_ref().map(|mark| mark_of(mark, row.conv_metres)) else {
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
            ordinal,
        })
    }

    fn gender_of(&mut self, team: &ParsedTeam, section: &ParsedSection) -> Gender {
        match team.gender.or(section.gender) {
            Some(gender) => gender,
            None => {
                self.stats.genders_unknown = self.stats.genders_unknown.saturating_add(1);
                Gender::Unknown
            }
        }
    }

    fn mint_row(
        &mut self,
        context: &ListContext<'_>,
        section: &ParsedSection,
        row: &ParsedRow,
        facts: &RowFacts<'_>,
        observed: (Grade, SchoolYear, Gender),
        school: &SchoolId,
    ) -> crate::CrawlResult<Option<()>> {
        let (grade, _, _) = observed;
        let source_key = format!(
            "{}:row:{}",
            source_key(facts.date, facts.meet, facts.athlete.id, section, row),
            facts.ordinal
        );
        let Some(mints) =
            self.mint_entities(context, section, facts, observed, school, &source_key)?
        else {
            return Ok(None);
        };
        let id = CanonicalPerformance::mint(
            &mints.athlete,
            &mints.meet,
            &mints.event,
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
            source_athlete: Some(mints.source_athlete),
            retained_conflicts: Vec::new(),
        };
        self.accumulator
            .performances
            .entry(id.as_str().to_string())
            .or_insert(performance);
        Ok(Some(()))
    }

    fn mint_entities(
        &mut self,
        context: &ListContext<'_>,
        section: &ParsedSection,
        facts: &RowFacts<'_>,
        observed: (Grade, SchoolYear, Gender),
        school: &SchoolId,
        source_key: &str,
    ) -> crate::CrawlResult<Option<RowMints>> {
        let (_, school_year, gender) = observed;
        let Some((grad_year, observation, source)) =
            admission::athlete(self, context, facts, observed, source_key)
        else {
            return Ok(None);
        };
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
        let (event, _) = self.event_for(context.page, section, &meet_id, gender)?;
        let (athlete_id, source_athlete) = self.athlete_for(
            context.page,
            AthleteFacts {
                school,
                name: facts.name,
                grad_year,
                gender,
                sport: facts.sport,
                source,
                observed_grade: Some(observation),
            },
        );
        Ok(Some(RowMints {
            team,
            meet: meet_id,
            event,
            athlete: athlete_id,
            source_athlete,
        }))
    }
}

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
