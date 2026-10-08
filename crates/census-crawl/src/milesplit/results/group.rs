use super::append;
use super::run::{owned_key, Run};
use super::EntityCounts;
use crate::milesplit::wire::ResultSetRef;
use crate::milesplit::ResultSetRequest;
use crate::{AdapterContext, CrawlResult};

pub(super) struct MeetGroup {
    pub(super) meet_id: String,
    pub(super) references: Vec<ResultSetRef>,
}

pub(super) fn group_meets(run: &mut Run, urls: &[ResultSetRequest]) -> Vec<MeetGroup> {
    let mut groups: Vec<MeetGroup> = Vec::new();
    for request in urls {
        match ResultSetRef::parse(&request.url)
            .or_else(|| ResultSetRef::parse_with_jurisdiction(&request.url, request.jurisdiction))
        {
            Some(reference) => push_reference(&mut groups, reference),
            None => run.reject(&request.url),
        }
    }
    groups
}

fn push_reference(groups: &mut Vec<MeetGroup>, reference: ResultSetRef) {
    let meet_id = owned_key(&reference);
    match groups.last_mut() {
        Some(group) if group.meet_id == meet_id => group.references.push(reference),
        _ => groups.push(MeetGroup {
            meet_id,
            references: vec![reference],
        }),
    }
}

pub(super) async fn run_group(
    ctx: &AdapterContext<'_>,
    run: &mut Run,
    group: MeetGroup,
    window_rows: usize,
    total: &mut EntityCounts,
) -> CrawlResult<()> {
    let MeetGroup {
        meet_id,
        references,
    } = group;
    run.meet_id = meet_id;
    for reference in &references {
        run.read(ctx, reference).await?;
        if run.rows() >= window_rows {
            absorb(
                total,
                append(ctx, run.drain_accumulated(), run.drain_pending())?,
            );
        }
    }
    absorb(
        total,
        append(ctx, run.drain_accumulated(), run.drain_pending())?,
    );
    run.release_owned();
    Ok(())
}

fn absorb(total: &mut EntityCounts, counts: EntityCounts) {
    total.meets = total.meets.saturating_add(counts.meets);
    total.events = total.events.saturating_add(counts.events);
    total.teams = total.teams.saturating_add(counts.teams);
    total.athletes = total.athletes.saturating_add(counts.athletes);
    total.performances = total.performances.saturating_add(counts.performances);
    total.unsupported_cohorts = total
        .unsupported_cohorts
        .saturating_add(counts.unsupported_cohorts);
}
