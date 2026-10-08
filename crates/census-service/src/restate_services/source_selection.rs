use super::jobs;
use census_crawl::registry::{descriptors, SourceDescriptor};
use restate_sdk::prelude::HandlerError;

#[derive(Clone, Copy)]
pub(super) struct SourceSelection(u64);

impl SourceSelection {
    pub(super) fn parse(slugs: &[String]) -> Result<Self, HandlerError> {
        if descriptors().count() > 64 {
            return Err(jobs::invariant(
                "source inventory exceeds its compiled selection bound",
            ));
        }
        slugs
            .iter()
            .try_fold(0_u64, |mask, slug| {
                let (index, _) = descriptors()
                    .enumerate()
                    .find(|(_, source)| source.slug == slug)
                    .ok_or_else(|| {
                        jobs::invariant(&format!("source plan contains unknown family {slug}"))
                    })?;
                let shift =
                    u32::try_from(index).map_err(|_| jobs::invariant("source index overflow"))?;
                let bit = 1_u64
                    .checked_shl(shift)
                    .ok_or_else(|| jobs::invariant("source mask overflow"))?;
                Ok(mask | bit)
            })
            .map(Self)
    }

    pub(super) fn iter(self) -> impl Iterator<Item = &'static SourceDescriptor> {
        descriptors()
            .take(64)
            .enumerate()
            .filter_map(move |(index, source)| {
                let bit = u32::try_from(index)
                    .ok()
                    .and_then(|shift| 1_u64.checked_shl(shift));
                bit.filter(|bit| self.0 & bit != 0).map(|_| source)
            })
    }
}
