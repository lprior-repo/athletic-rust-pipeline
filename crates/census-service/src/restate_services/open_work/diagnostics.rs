use super::{JurisdictionOpen, ReadJurisdiction};

pub(super) fn diagnostic_row(mut entry: ReadJurisdiction) -> JurisdictionOpen {
    if !entry.row.unreadable {
        entry.row.teams_stage = Some(entry.state.teams);
        entry.row.source_refusals = entry.state.plan.map(|plan| plan.refused);
    }
    entry.row
}
