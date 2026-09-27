use crate::AdapterReport;

#[derive(Debug, Default)]
pub(in crate::athleticnet) struct MeetStats {
    pub(in crate::athleticnet) meets_pulled: u64,
    pub(in crate::athleticnet) meets_unplaced: u64,
    pub(in crate::athleticnet) meets_without_date: u64,
    pub(in crate::athleticnet) meets_without_season: u64,
    pub(super) blocks: u64,
    pub(super) blocks_gender_unknown: u64,
    pub(super) blocks_without_division: u64,
    pub(super) blocks_event_type_mismatch: u64,
    pub(super) blocks_metadata_absent: u64,
    pub(super) rows_seen: u64,
    pub(super) rows_stored: u64,
    pub(super) rows_no_mark: u64,
    pub(super) rows_no_name: u64,
    pub(super) rows_no_athlete: u64,
    pub(super) rows_no_grade: u64,
    pub(super) rows_unknown_school: u64,
    pub(super) rows_unmapped_event: u64,
    pub(super) relay_rows: u64,
    pub(super) relay_rows_without_legs: u64,
    pub(super) legs_seen: u64,
    pub(super) legs_stored: u64,
    pub(super) legs_no_name: u64,
    pub(super) legs_no_athlete: u64,
    pub(super) legs_no_grade: u64,
}

impl MeetStats {
    pub(in crate::athleticnet) fn stored(&self) -> u64 {
        self.rows_stored.saturating_add(self.legs_stored)
    }

    pub(in crate::athleticnet) fn merge(&mut self, other: &MeetStats) {
        self.meets_pulled = self.meets_pulled.saturating_add(other.meets_pulled);
        self.meets_unplaced = self.meets_unplaced.saturating_add(other.meets_unplaced);
        self.meets_without_date = self
            .meets_without_date
            .saturating_add(other.meets_without_date);
        self.meets_without_season = self
            .meets_without_season
            .saturating_add(other.meets_without_season);
        self.blocks = self.blocks.saturating_add(other.blocks);
        self.blocks_gender_unknown = self
            .blocks_gender_unknown
            .saturating_add(other.blocks_gender_unknown);
        self.blocks_without_division = self
            .blocks_without_division
            .saturating_add(other.blocks_without_division);
        self.blocks_event_type_mismatch = self
            .blocks_event_type_mismatch
            .saturating_add(other.blocks_event_type_mismatch);
        self.blocks_metadata_absent = self
            .blocks_metadata_absent
            .saturating_add(other.blocks_metadata_absent);
        self.rows_seen = self.rows_seen.saturating_add(other.rows_seen);
        self.rows_stored = self.rows_stored.saturating_add(other.rows_stored);
        self.rows_no_mark = self.rows_no_mark.saturating_add(other.rows_no_mark);
        self.rows_no_name = self.rows_no_name.saturating_add(other.rows_no_name);
        self.rows_no_athlete = self.rows_no_athlete.saturating_add(other.rows_no_athlete);
        self.rows_no_grade = self.rows_no_grade.saturating_add(other.rows_no_grade);
        self.rows_unknown_school = self
            .rows_unknown_school
            .saturating_add(other.rows_unknown_school);
        self.rows_unmapped_event = self
            .rows_unmapped_event
            .saturating_add(other.rows_unmapped_event);
        self.relay_rows = self.relay_rows.saturating_add(other.relay_rows);
        self.relay_rows_without_legs = self
            .relay_rows_without_legs
            .saturating_add(other.relay_rows_without_legs);
        self.legs_seen = self.legs_seen.saturating_add(other.legs_seen);
        self.legs_stored = self.legs_stored.saturating_add(other.legs_stored);
        self.legs_no_name = self.legs_no_name.saturating_add(other.legs_no_name);
        self.legs_no_athlete = self.legs_no_athlete.saturating_add(other.legs_no_athlete);
        self.legs_no_grade = self.legs_no_grade.saturating_add(other.legs_no_grade);
    }
}

impl std::fmt::Display for MeetStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "meets: {} pulled, {} whose payload places the venue in no jurisdiction, {} without a \
             published date, {} without a season; blocks: {} read, {} without a published gender, \
             {} without a division, {} listed by no metadata document, {} whose label disagrees \
             with the declared event type; rows: {} seen, {} stored, {} without a mark token, {} \
             without a name, {} without an athlete id, {} without a grade, {} whose team id has no \
             entry, {} whose event label maps to no platform kind; relays: {} squad rows, {} \
             without leg membership, {} legs seen, {} stored, {} without a name, {} without an \
             athlete id, {} without a grade",
            self.meets_pulled,
            self.meets_unplaced,
            self.meets_without_date,
            self.meets_without_season,
            self.blocks,
            self.blocks_gender_unknown,
            self.blocks_without_division,
            self.blocks_metadata_absent,
            self.blocks_event_type_mismatch,
            self.rows_seen,
            self.rows_stored,
            self.rows_no_mark,
            self.rows_no_name,
            self.rows_no_athlete,
            self.rows_no_grade,
            self.rows_unknown_school,
            self.rows_unmapped_event,
            self.relay_rows,
            self.relay_rows_without_legs,
            self.legs_seen,
            self.legs_stored,
            self.legs_no_name,
            self.legs_no_athlete,
            self.legs_no_grade
        )
    }
}

pub(in crate::athleticnet) fn note(
    report: &mut AdapterReport,
    totals: &MeetStats,
    metadata_spent: bool,
) {
    report.note(format!("total {totals}"));
    if metadata_spent {
        report.note(
            "metadata: the third request (`Meet/GetEventDivisionData`) was spent, so every block's \
             label was cross-checked against the declared event type",
        );
    } else {
        report.note(
            "metadata: not spent (the third request is off by default), so an event whose own \
             label maps to no platform kind had its marks refused rather than guessed at",
        );
    }
}
