use super::*;
use crate::net::FetchOutcome;
use crate::recording::plan;
use serde::{Deserialize, Serialize};

mod capture;

#[derive(Serialize, Deserialize)]
struct Schedule {
    sport: ScheduleSport,
    year: i16,
    capture: capture::Capture,
}

pub async fn collect_fragment(
    ctx: &AdapterContext<'_>,
    options: &Options,
    ordinal: usize,
) -> CrawlResult<(AdapterReport, bool)> {
    let before = stats_of(ctx).await;
    let mut walk = Walk::new(Lane::of(ctx));
    let mut offset = ordinal;
    for schedule in schedules(ctx, options).await? {
        let url = schedule_url(schedule.sport, schedule.year);
        if offset >= schedule.capture.rows {
            offset = offset
                .checked_sub(schedule.capture.rows)
                .ok_or_else(|| budget::arithmetic("Wayzata fragment cursor"))?;
            continue;
        }
        let fetched = capture::read(ctx, schedule.capture)?;
        let body = validate_page(&fetched, &url)?;
        let rows = super::super::parse::rows(body, schedule.year)?;
        let row = rows
            .get(offset)
            .ok_or_else(|| budget::arithmetic("Wayzata fragment row"))?;
        let page = Page {
            fetched: &fetched,
            sport: schedule.sport,
            year: schedule.year,
        };
        let row_ordinal = offset
            .checked_add(1)
            .ok_or_else(|| budget::arithmetic("Wayzata fragment ordinal"))?;
        walk.pages = 1;
        apply(&mut walk, ctx, options, &page, row_ordinal, row).await?;
        return Ok((walk.finish(ctx, before).await?, true));
    }
    Ok((walk.finish(ctx, before).await?, false))
}

async fn apply(
    walk: &mut Walk,
    ctx: &AdapterContext<'_>,
    options: &Options,
    page: &Page<'_>,
    ordinal: usize,
    row: &MeetRow,
) -> CrawlResult<()> {
    match walk.apply_row(ctx, options, page, ordinal, row).await {
        Ok(Some(meet)) if walk.lane.dispatch() => {
            if let Err(error) = walk.read_results(ctx, options, page, row, &meet).await {
                walk.failed(locator(page, ordinal), error)?;
            }
        }
        Ok(_) => {}
        Err(error) => walk.failed(locator(page, ordinal), error)?,
    }
    Ok(())
}

async fn schedules(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<Vec<Schedule>> {
    if let Some(schedules) = plan::read(ctx, "wayzata-schedules")? {
        return Ok(schedules);
    }
    let years = super::super::calendar_years(ctx, options)?;
    let pages = [ScheduleSport::Track, ScheduleSport::CrossCountry]
        .into_iter()
        .flat_map(|sport| years.iter().copied().map(move |year| (sport, year)));
    let schedules = stream::iter(pages.map(Ok))
        .try_fold(Vec::new(), |mut schedules, (sport, year)| async move {
            let capture = capture::freeze(ctx, options, sport, year).await?;
            schedules
                .try_reserve(1)
                .map_err(|_| budget::arithmetic("Wayzata frozen schedules allocation"))?;
            schedules.push(Schedule {
                sport,
                year,
                capture,
            });
            Ok::<_, CrawlError>(schedules)
        })
        .await?;
    plan::write(ctx, "wayzata-schedules", &schedules)?;
    Ok(schedules)
}

#[cfg(test)]
mod tests;
