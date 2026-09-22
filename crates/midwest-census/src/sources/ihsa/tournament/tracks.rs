//! Track & field: the meet and event index rows, and one summary's finisher rows.
//!
//! The index path and the summary path mint the same event identity: the class and the round are read
//! from the event row's own `classDivision` and `round` fields, so the published name only has to
//! yield the event itself. An individual row hangs one performance on the athlete it names; a relay
//! row publishes `members[]` instead — four legs carrying their own Athletic.net ids — and its own mark
//! is the team's, not any leg's, so the legs are minted as athletes and no performance is invented for
//! a mark no single athlete ran.
//!
//! Nothing reaches the store from here: rows land in the run's accumulator and are appended once, at
//! the end of the walk.

use super::map::{published_gender, unmapped, AthleteRow, EventContext, Mapper, PerformanceRow};
use super::parse::{
    class_token, event_label, finisher_grade, member_grade, parse_mark, round_label,
};
use super::wire::{EventRow, EventSummary, FinisherRow, MeetRow};
use census_domain::model::{
    AthleteId, CanonicalEvent, CanonicalMeet, CompetitionLevel, EventKind, Mark, SchoolId,
    SourceEventLabel, SourceIdentity, SourceNamespace, Sport,
};
use census_domain::UsJurisdiction;

impl<'a> Mapper<'a> {
    /// The meet one index row publishes, kept once its events are known.
    ///
    /// `MeetId` is the Athletic.net Live meet id — the index publishes `74003`, the page is
    /// `https://live.athletic.net/meets/74003` — so the meet carries that identity and the page.
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

    /// The event one index row publishes.
    ///
    /// The event's own fields carry the class and the round, so the published name only has to yield
    /// the event itself; a name the ontology cannot place is kept as `Unmapped` with that name.
    pub(super) fn event(
        &mut self,
        meet: &CanonicalMeet,
        row: &EventRow,
        url: &str,
    ) -> CanonicalEvent {
        let kind = event_label(&row.event_name, &row.class_division)
            .map_or_else(|| unmapped(&row.event_name), EventKind::from_source_label);
        let mut event = CanonicalEvent::new(
            &meet.id,
            kind,
            published_gender(&row.gender),
            class_token(&row.class_division),
            round_label(row.round.as_deref()),
        );
        event.source_labels.push(SourceEventLabel {
            source: self.origin.source(url),
            label: row.event_name.clone(),
        });
        event.evidence.push(self.origin.evidence(url));
        self.accumulated
            .events
            .entry(event.id.as_str().to_string())
            .or_insert_with(|| event.clone());
        event
    }

    /// Map one event summary's finisher rows.
    pub(super) fn absorb_summary(
        &mut self,
        summary: &EventSummary,
        context: &EventContext<'_>,
        url: &str,
    ) {
        for row in &summary.finishers {
            self.stats.rows = self.stats.rows.saturating_add(1);
            if row.members.is_empty() {
                self.individual(row, context, url);
            } else {
                self.relay(row, context, url);
            }
        }
    }

    /// Store one placed individual's performance.
    ///
    /// The row is dropped, and counted, when the payload leaves out a field the performance cannot be
    /// keyed or valued by: a school it names, a grade, or a mark.
    fn individual(&mut self, row: &FinisherRow, context: &EventContext<'_>, url: &str) {
        let Some(school) =
            self.school(row.ihsa_school_id.as_deref(), row.team_name.as_deref(), url)
        else {
            return;
        };
        let reference = row.athlete.as_ref();
        let name = reference
            .and_then(|who| who.name.as_deref())
            .or(row.athlete_name.as_deref());
        let Some(athlete) = self.athlete(
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
        self.place(&school, &athlete, mark, row, context, url);
    }

    /// Store the performance of one individual row that has an athlete, a school and a mark.
    fn place(
        &mut self,
        school: &SchoolId,
        athlete: &AthleteId,
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
        let source_key = format!("{}:{}", context.event.id.as_str(), athlete.as_str());
        self.performance(
            athlete,
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

    /// Mint a relay row's legs: four athletes, and no performance row for the team's own mark.
    fn relay(&mut self, row: &FinisherRow, context: &EventContext<'_>, url: &str) {
        let Some(school) =
            self.school(row.ihsa_school_id.as_deref(), row.team_name.as_deref(), url)
        else {
            return;
        };
        self.team(
            &school,
            context.sport,
            context.event.gender,
            context.school_year,
            row.team.as_ref(),
            url,
        );
        for member in &row.members {
            let Some(leg) = member.athlete.as_ref() else {
                continue;
            };
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
                },
                self.origin.evidence(url),
            );
            if stored.is_none() {
                self.stats.rows_no_grade = self.stats.rows_no_grade.saturating_add(1);
            } else {
                self.stats.legs = self.stats.legs.saturating_add(1);
            }
        }
    }
}
