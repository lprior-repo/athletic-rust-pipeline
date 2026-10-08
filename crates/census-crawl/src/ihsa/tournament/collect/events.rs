use super::super::map::{school_year_of, EventContext};
use super::super::parse;
use super::super::requests::{self, summary_url};
use super::super::wire::{EventRow, EventSummary};
use super::Run;
use census_domain::model::{CanonicalMeet, Sport};

impl Run<'_> {
    pub(super) async fn walk_events(
        &mut self,
        rows: &[EventRow],
        request: (u64, &str),
        meet: &CanonicalMeet,
        date: &str,
    ) -> (usize, bool) {
        let (meet_id, index_url) = request;
        let mut complete = true;
        let index_capture = self.mapper.capture.clone();
        let index_stamp = self.mapper.origin.observed_on.clone();
        for row in rows {
            self.mapper.capture = index_capture.clone();
            self.mapper.origin.observed_on.clone_from(&index_stamp);
            let event = match self.mapper.event(meet, row, index_url) {
                Ok(event) => event,
                Err(error) => {
                    self.refuse_event(index_url, &row.event_id, &error);
                    complete = false;
                    continue;
                }
            };
            self.mapper.count_event(row.has_results);
            if row.has_results {
                let context = self.event_context(row, meet, &event, date);
                if !self.read_summary(row, meet_id, &context).await {
                    complete = false;
                }
            }
        }
        (rows.len(), complete)
    }

    fn refuse_event(&mut self, url: &str, event: &str, error: &crate::CrawlError) {
        self.report.errors = self.report.errors.saturating_add(1);
        self.report.note(format!("{url}: event {event}: {error}"));
        self.report.unfinished.push(format!("{url}#event={event}"));
    }

    fn event_context<'b>(
        &self,
        row: &'b EventRow,
        meet: &'b CanonicalMeet,
        event: &'b census_domain::model::CanonicalEvent,
        date: &'b str,
    ) -> EventContext<'b> {
        let date = row
            .scheduled_date
            .as_deref()
            .map(|published| parse::date_part(published).map_or(published, |value| value))
            .map_or(date, |value| value);
        EventContext {
            meet,
            event,
            sport: Sport::OutdoorTrack,
            date,
            school_year: school_year_of(date, self.ctx.school_year),
            performance_as_of: self.ctx.performance_as_of,
        }
    }

    async fn read_summary(
        &mut self,
        row: &EventRow,
        meet_id: u64,
        context: &EventContext<'_>,
    ) -> bool {
        let url = summary_url(&row.event_id);
        let Some((summary, capture)) = requests::summary(self.ctx, &mut self.report, &url).await
        else {
            return false;
        };
        if !summary_matches(&summary, row, meet_id) {
            self.refuse(&url, "summary contradicts the requested event context");
            return false;
        }
        if let Err(error) = self.mapper.bind_capture(capture) {
            self.refuse(&url, &error.to_string());
            return false;
        }
        match self.ctx.assess_performance_date(context.date) {
            crate::context::PerformanceDateAssessment::Future => return true,
            crate::context::PerformanceDateAssessment::Unknown => {
                let error = crate::CrawlError::PerformanceDateUnknown {
                    published: context.date.chars().take(64).collect(),
                    as_of: context.performance_as_of,
                };
                self.refuse(&url, &format!("{error}; original rows retained in capture"));
                return false;
            }
            crate::context::PerformanceDateAssessment::Admitted => {}
        }
        if census_domain::model::SchoolYear::from_date(context.date).is_none() {
            self.refuse(&url, "published date has no supported academic period");
            return false;
        }
        self.mapper.absorb_summary(&summary, context, &url);
        self.summaries = self.summaries.saturating_add(1);
        true
    }
}

fn summary_matches(summary: &EventSummary, row: &EventRow, meet_id: u64) -> bool {
    summary.meet_id == meet_id
        && summary.event_id == row.event_id
        && summary.event_type == row.event_type
        && summary.gender == row.gender
        && summary.class_division == row.class_division
        && summary.event_name == row.event_name
        && summary.round == row.round
        && summary
            .round_label
            .as_deref()
            .is_none_or(|label| Some(label) == parse::round_label(row.round.as_deref()))
        && summary.scheduled_date == row.scheduled_date
        && summary.has_results == row.has_results
}
