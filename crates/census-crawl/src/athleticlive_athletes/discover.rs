use super::{meet_targets, MeetTarget, Options};
use crate::directory::acquisition::{bounded, owe};
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::CanonicalMeet;
use census_store::Table;

pub(super) fn discover(
    ctx: &AdapterContext<'_>,
    options: &Options,
    report: &mut AdapterReport,
) -> CrawlResult<Vec<MeetTarget>> {
    let mut targets = Vec::new();
    let mut failure = None;
    ctx.store
        .for_each_merged::<CanonicalMeet>(Table::Meets, |meet| {
            let locator = format!("store:meets#{}", meet.id);
            let selection = meet_targets(std::slice::from_ref(&meet), &options.states);
            if failure.is_none() {
                let result = (|| {
                    if selection.skipped_implausible > 0 || selection.skipped_unplaced > 0 {
                        owe(report, &locator)?;
                    }
                    selection
                        .targets
                        .into_iter()
                        .try_for_each(|target| bounded(&mut targets, target))
                })();
                if let Err(error) = result {
                    failure = Some((locator, error));
                }
            }
            Ok(())
        })?;
    if let Some((locator, error)) = failure {
        crate::directory::acquisition::fail(report, &locator, error)?;
    }
    if targets.is_empty() {
        owe(report, "store:meets#athleticlive-discovery")?;
    }
    Ok(targets)
}
