use super::entities::team_identities;
use super::map::{AthleteRow, EventContext, Mapper, PerformanceRow};
use super::parse::{finisher_grade, member_grade, parse_mark};
use super::wire::{EventRow, EventSummary, FinisherRow, MeetRow};
use census_domain::model::{
    AthleteId, CanonicalEvent, CanonicalMeet, CompetitionLevel, Mark, RelayMember, RelayResult,
    SchoolId, SourceIdentity, SourceNamespace, Sport,
};
use census_domain::UsJurisdiction;
mod events;

impl<'a> Mapper<'a> {
    pub(super) fn meet(
        &mut self,
        row: &MeetRow,
        date: &str,
        end_date: Option<&str>,
        url: &str,
    ) -> CanonicalMeet {
        let mut meet = CanonicalMeet::new(
            Some(UsJurisdiction::Illinois),
            row.title.clone(),
            date,
            CompetitionLevel::State,
        );
        meet.end_date = end_date.filter(|end| *end != date).map(str::to_string);
        meet.sports.push(Sport::OutdoorTrack);
        let page = format!("https://live.athletic.net/meets/{}", row.meet_id);
        meet.source_identities.push(SourceIdentity {
            namespace: SourceNamespace::AthleticNet {
                kind: "live".to_string(),
            },
            id: row.meet_id.to_string(),
            url: Some(page.clone()),
        });
        meet.source_urls.push(page);
        meet.evidence.push(self.origin.evidence(url));
        self.accumulated
            .meets
            .entry(meet.id.as_str().to_string())
            .or_insert_with(|| meet.clone());
        meet
    }

    pub(super) fn event(
        &mut self,
        meet: &CanonicalMeet,
        row: &EventRow,
        url: &str,
    ) -> crate::CrawlResult<CanonicalEvent> {
        let mut event = events::mint(meet, row)?;
        events::retain(self, &mut event, &row.event_name, url)?;
        Ok(event)
    }

    pub(super) fn absorb_summary(
        &mut self,
        summary: &EventSummary,
        context: &EventContext<'_>,
        url: &str,
    ) {
        for (row_index, row) in summary.finishers.iter().enumerate() {
            self.stats.rows = self.stats.rows.saturating_add(1);
            if row.members.is_empty() {
                self.individual(row, context, url, row_index);
            } else {
                self.relay(row, context, url, row_index);
            }
        }
    }

    fn individual(
        &mut self,
        row: &FinisherRow,
        context: &EventContext<'_>,
        url: &str,
        row_index: usize,
    ) {
        let Some(school) =
            self.school(row.ihsa_school_id.as_deref(), row.team_name.as_deref(), url)
        else {
            return;
        };
        let reference = row.athlete.as_ref();
        let name = reference
            .and_then(|who| who.name.as_deref())
            .or(row.athlete_name.as_deref());
        let Some(subject) = self.athlete(
            AthleteRow {
                name,
                grade: finisher_grade(row),
                gender: context.event.gender,
                school: &school,
                sport: context.sport,
                school_year: context.school_year,
                net_id: reference.and_then(|who| who.athletic_net_id),
                live_id: reference.and_then(|who| who.athletic_live_id),
                entry: None,
                source_key: format!("{url}:row:{row_index}"),
            },
            self.origin.evidence(url),
        ) else {
            self.stats.rows_no_grade = self.stats.rows_no_grade.saturating_add(1);
            return;
        };
        let Some(mark) = row.mark.as_deref().and_then(parse_mark) else {
            self.stats.rows_no_mark = self.stats.rows_no_mark.saturating_add(1);
            return;
        };
        self.place(&school, subject, mark, row, context, url);
    }

    fn place(
        &mut self,
        school: &SchoolId,
        subject: (AthleteId, SourceIdentity),
        mark: Mark,
        row: &FinisherRow,
        context: &EventContext<'_>,
        url: &str,
    ) {
        let team = self.team(
            school,
            context.sport,
            context.event.gender,
            context.school_year,
            row.team.as_ref(),
            url,
        );
        let source_key = format!("{}:{}", context.event.id.as_str(), subject.0.as_str());
        self.performance(
            subject,
            &team,
            context,
            PerformanceRow {
                date: context.date,
                mark,
                place: row.place,
                grade: finisher_grade(row),
                source_key,
            },
            url,
        );
    }

    fn relay(
        &mut self,
        row: &FinisherRow,
        context: &EventContext<'_>,
        url: &str,
        row_index: usize,
    ) {
        let Some(school) =
            self.school(row.ihsa_school_id.as_deref(), row.team_name.as_deref(), url)
        else {
            return;
        };
        let team = self.team(
            &school,
            context.sport,
            context.event.gender,
            context.school_year,
            row.team.as_ref(),
            url,
        );
        let mut members = Vec::new();
        for (leg_index, member) in row.members.iter().enumerate() {
            let Some(leg) = member.athlete.as_ref() else {
                continue;
            };
            let order = member
                .order
                .unwrap_or_else(|| u32::try_from(leg_index.saturating_add(1)).unwrap_or(u32::MAX));
            let mut published = RelayMember::new(order, leg.name.clone().unwrap_or_default());
            let stored = self.athlete(
                AthleteRow {
                    name: leg.name.as_deref(),
                    grade: member_grade(member),
                    gender: context.event.gender,
                    school: &school,
                    sport: context.sport,
                    school_year: context.school_year,
                    net_id: leg.athletic_net_id,
                    live_id: leg.athletic_live_id,
                    entry: None,
                    source_key: format!("{url}:row:{row_index}:leg:{leg_index}"),
                },
                self.origin.evidence(url),
            );
            match stored {
                Some((athlete_id, _)) => {
                    self.stats.legs = self.stats.legs.saturating_add(1);
                    published = published.with_athlete(athlete_id);
                }
                None => {
                    self.stats.rows_no_grade = self.stats.rows_no_grade.saturating_add(1);
                }
            }
            members.push(published);
        }
        let Some(mark) = row.mark.as_deref().and_then(parse_mark) else {
            self.stats.rows_no_mark = self.stats.rows_no_mark.saturating_add(1);
            return;
        };
        let mut relay = RelayResult::new(
            &team,
            &context.event.id,
            &context.meet.id,
            mark,
            format!("{url}:row:{row_index}"),
        )
        .with_place(row.place)
        .with_round(context.event.round.clone())
        .with_evidence(self.origin.evidence(url));
        if let Some(identity) = team_identities(row.team.as_ref()).into_iter().next() {
            relay = relay.with_source_team(identity);
        }
        for member in members {
            relay = relay.with_member(member);
        }
        self.accumulated
            .relay_results
            .entry(relay.id.as_str().to_owned())
            .or_insert(relay);
    }
}
