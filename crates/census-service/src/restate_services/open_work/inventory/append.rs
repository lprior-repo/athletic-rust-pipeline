use super::super::{history, obligation, push, ReadJurisdiction};
use super::plan_covers;
use crate::restate_services::wire::SourceObjectOpen;
use census_crawl::CollectionDisposition as Disposition;
use restate_sdk::prelude::*;

pub(super) fn append_plan(
    entry: &ReadJurisdiction,
    rows: &mut Vec<SourceObjectOpen>,
) -> Result<(), HandlerError> {
    if !plan_covers(entry.row.jurisdiction, &entry.state) {
        push(
            rows,
            obligation(
                format!("{}/source-plan/incomplete-inventory", entry.row.identity),
                Disposition::Unknown,
            ),
        )?;
    }
    let Some(plan) = &entry.state.plan else {
        return push(
            rows,
            obligation(
                format!("{}/source-plan/unmeasured", entry.row.identity),
                Disposition::Unknown,
            ),
        );
    };
    plan.refused.iter().try_for_each(|refusal| {
        push(
            rows,
            obligation(
                format!(
                    "{}/refused/{}/{:?}",
                    entry.row.identity, refusal.slug, refusal.kind
                ),
                Disposition::Blocked,
            ),
        )
    })
}

pub(super) fn append_history(
    entry: &ReadJurisdiction,
    slug: &str,
    rows: &mut Vec<SourceObjectOpen>,
) -> Result<(), HandlerError> {
    let Some(window) = entry.state.history_window else {
        return push(
            rows,
            obligation(
                format!("{}/history/{slug}/unmeasured", entry.row.identity),
                Disposition::Unknown,
            ),
        );
    };
    [history::Kind::Meets, history::Kind::Results]
        .into_iter()
        .filter(|kind| kind.requires(slug))
        .try_for_each(|kind| {
            window.years().try_for_each(|year| {
                let (disposition, observations, resolution) =
                    history::source(&entry.state, year, slug, kind)?;
                let mut row = obligation(
                    format!(
                        "{}/history/{year}/{}/{slug}",
                        entry.row.identity,
                        kind.name()
                    ),
                    disposition,
                );
                row.observations = observations;
                row.resolution = resolution;
                row.windows = u64::from(disposition.is_complete());
                push(rows, row)
            })
        })
}
