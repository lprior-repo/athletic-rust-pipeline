use super::*;

impl MeetCtx<'_> {
    pub(super) fn build_relay_result(
        &self,
        team: &TeamId,
        event: &EventId,
        mark: Mark,
        source_key: String,
        row: &FlatRow,
        round: Option<String>,
        auto: bool,
        retained_conflicts: Vec<census_domain::model::RetainedConflict>,
    ) -> RelayResult {
        let timing = if matches!(mark, Mark::Raw(_)) {
            None
        } else {
            timing_of(0, auto)
        };
        let mut relay = RelayResult::new(team, event, &self.meet.id, mark, source_key)
            .with_place(
                row.place
                    .as_deref()
                    .and_then(|place| place.trim().parse().ok()),
            )
            .with_round(round)
            .with_timing(timing)
            .with_evidence(census_domain::model::Evidence::parsed(
                self.source.clone(),
                self.observed_on,
            ));
        if let Some(team_id) = row.team_id {
            relay = relay.with_source_team(SourceIdentity::new(
                SourceNamespace::athletic_net("team"),
                team_id.to_string(),
            ));
        }
        relay.retained_conflicts = retained_conflicts;
        relay
    }
    pub(super) fn resolve_relay_team(
        &mut self,
        row: &FlatRow,
        school: &SchoolId,
        gender: Gender,
    ) -> TeamId {
        let team = ensure_team_for(
            self.accumulated,
            self.source,
            self.observed_on,
            school,
            self.sport,
            gender,
            self.school_year,
        );
        if let Some(team_entry) = self.accumulated.teams.get_mut(team.as_str()) {
            if let Some(team_id) = row.team_id {
                let identity =
                    SourceIdentity::new(SourceNamespace::athletic_net("team"), team_id.to_string());
                if !team_entry.source_identities.contains(&identity) {
                    team_entry.source_identities.push(identity);
                }
            }
        }
        team
    }
    pub(super) fn project_relay_event(
        &mut self,
        block: &Block<'_>,
        row: &FlatRow,
        mark: Mark,
        auto: bool,
    ) -> crate::CrawlResult<(
        Mark,
        bool,
        EventId,
        Vec<census_domain::model::RetainedConflict>,
    )> {
        let event_input = EventProjectionInput {
            meet: &self.meet,
            kind: block.kind,
            gender: block.gender,
            division: block.division.as_deref(),
            round: block.round.as_deref(),
            labels: block.labels,
        };
        match super::super::events::project_event(
            self.accumulated,
            self.source,
            self.observed_on,
            &event_input,
            block.metadata_conflict,
        )? {
            super::super::events::ProjectionOutcome::Admitted(event) => {
                Ok((mark, auto, event, Vec::new()))
            }
            super::super::events::ProjectionOutcome::Withheld(event, conflict) => {
                let mark = if !matches!(mark, Mark::Raw(_)) {
                    Mark::Raw(row.result.clone())
                } else {
                    mark
                };
                Ok((mark, auto, event, vec![conflict]))
            }
        }
    }
}
