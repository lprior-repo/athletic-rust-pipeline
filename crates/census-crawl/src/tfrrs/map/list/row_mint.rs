use super::{Absorb, ListContext, row_facts::RowFacts, admission, TeamFacts, AthleteFacts};
use super::super::row::source_key;
use crate::tfrrs::parse::{parse_team_path, ParsedRow, ParsedSection};
use census_domain::model::{
    AthleteId, CanonicalPerformance, EventId, Gender, Grade, MeetId, SchoolId, SchoolYear, SourceIdentity, TeamId,
};
use census_store::Entity;

pub(super) struct RowMints {
    pub(super) team: TeamId,
    pub(super) meet: MeetId,
    pub(super) event: EventId,
    pub(super) athlete: AthleteId,
    pub(super) source_athlete: SourceIdentity,
}

impl<'a> Absorb<'a> {
    fn same_observation(existing: &CanonicalPerformance, candidate: &CanonicalPerformance) -> bool {
        existing.mark == candidate.mark
            && existing.adjustment == candidate.adjustment
            && existing.wind_mps == candidate.wind_mps
            && existing.place == candidate.place
            && existing.timing == candidate.timing
            && existing.observed_grade == candidate.observed_grade
    }

    pub(super) fn mint_row(
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
            adjustment: facts.adjustment.clone(),
            wind_mps: row.wind,
            place: row.place,
            heat: None,
            round: None,
            timing: None,
            observed_grade: Some(grade),
            evidence: super::row_evidence(context, row),
            source_key,
            source_athlete: Some(mints.source_athlete),
            retained_conflicts: Vec::new(),
        };
        let key = id.as_str().to_string();
        if let Some(existing) = self.accumulator.performances.get_mut(&key) {
            if !Self::same_observation(existing, &performance) {
                existing.merge(performance);
            }
        } else {
            self.accumulator.performances.insert(key, performance);
        }
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
        let meet_id = self.meet_for(context.page, facts.meet, facts.date, facts.sport)?;
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