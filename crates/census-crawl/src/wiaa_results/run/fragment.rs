use super::super::{archive_artifacts, stats_of, ArchiveArtifact, Options, ARCHIVES};
use super::{append_entities, consolidated_schools, finish_report, process_artifact, ArtifactRun};
use crate::recording::plan;
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{CanonicalSchool, Sport};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Serialize, Deserialize)]
struct Catalogue {
    entries: Vec<(ArchiveArtifact, Sport)>,
    empty: Vec<String>,
}

pub async fn collect_fragment(
    ctx: &AdapterContext<'_>,
    options: &Options,
    ordinal: usize,
) -> CrawlResult<(AdapterReport, bool)> {
    let before = stats_of(ctx).await;
    let mut report = AdapterReport::new("wiaa_results", "artifacts");
    let catalogue = catalogue(ctx, options).await?;
    report.unfinished = catalogue.empty;
    let Some((artifact, sport)) = catalogue.entries.into_iter().nth(ordinal) else {
        report.finish_frontier();
        return Ok((report, false));
    };
    let schools = schools(ctx)?;
    let mut run = ArtifactRun::new(
        &schools,
        HashSet::new(),
        super::run_receipts::context(ctx, &schools)?,
    );
    run.stats.artifacts_seen = 1;
    process_artifact(ctx, &mut report, &mut run, &artifact, sport).await?;
    let counts = append_entities(ctx, run.accumulated, run.pending)?;
    run.counts.absorb(counts)?;
    finish_report(ctx, &mut report, &run.stats, run.counts, before.0, before.1).await?;
    report.finish_frontier();
    Ok((report, true))
}

fn schools(ctx: &AdapterContext<'_>) -> CrawlResult<Vec<CanonicalSchool>> {
    if let Some(schools) = plan::read(ctx, "wiaa-schools")? {
        return Ok(schools);
    }
    let schools = consolidated_schools(ctx)?;
    plan::write(ctx, "wiaa-schools", &schools)?;
    Ok(schools)
}

async fn catalogue(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<Catalogue> {
    if let Some(catalogue) = plan::read(ctx, "wiaa-catalogue")? {
        return Ok(catalogue);
    }
    let mut catalogue = Catalogue {
        entries: Vec::new(),
        empty: Vec::new(),
    };
    let mut seen = HashSet::new();
    for (url, sport) in ARCHIVES {
        let fetched = ctx.fetcher.get(url, &ctx.fetch_options()).await?;
        let artifacts = archive_artifacts(&fetched.text())?;
        if artifacts.is_empty() {
            catalogue
                .empty
                .push(format!("{url}: no recognized result artifact catalogue"));
        }
        for artifact in artifacts {
            if (!options.seasons.is_empty() && !options.seasons.contains(&artifact.year))
                || seen.contains(&artifact.url)
            {
                continue;
            }
            if catalogue.entries.len() >= crate::recording::MAX_RECORDED_WORK {
                return Err(CrawlError::Resource {
                    resource: "WIAA frozen artifact catalogue",
                    requested: catalogue.entries.len().saturating_add(1),
                    limit: crate::recording::MAX_RECORDED_WORK,
                });
            }
            catalogue
                .entries
                .try_reserve(1)
                .map_err(|_| CrawlError::Invariant {
                    detail: "allocating WIAA frozen catalogue".into(),
                })?;
            seen.insert(artifact.url.clone());
            catalogue.entries.push((artifact, sport));
        }
    }
    plan::write(ctx, "wiaa-catalogue", &catalogue)?;
    Ok(catalogue)
}
