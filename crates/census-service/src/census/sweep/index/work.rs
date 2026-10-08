use super::{CrawlError, CrawlResult};
const LEGACY: &str = "legacy-inventory-unmeasured";
const MAX_LOCATORS: usize = 20_001;
const MAX_BYTES: usize = 4 * 1024 * 1024;

pub(super) fn merge(prior: Vec<String>, mut current: Vec<String>) -> CrawlResult<Vec<String>> {
    if prior.iter().any(|locator| locator == LEGACY)
        && !current.iter().any(|locator| locator == LEGACY)
    {
        current
            .try_reserve_exact(1)
            .map_err(|_| resource(current.len()))?;
        current.push(LEGACY.to_string());
    }
    validate(&current)?;
    Ok(current)
}

pub(super) fn validate(locators: &[String]) -> CrawlResult<()> {
    if locators.len() > MAX_LOCATORS {
        return Err(resource(locators.len()));
    }
    locators.iter().try_fold(0usize, |bytes, locator| {
        let bytes = bytes
            .checked_add(locator.len())
            .ok_or_else(|| CrawlError::Arithmetic {
                detail: "index unfinished byte overflow".to_string(),
            })?;
        if bytes > MAX_BYTES {
            return Err(CrawlError::Resource {
                resource: "index unfinished bytes",
                requested: bytes,
                limit: MAX_BYTES,
            });
        }
        Ok(bytes)
    })?;
    Ok(())
}
fn resource(requested: usize) -> CrawlError {
    CrawlError::Resource {
        resource: "index unfinished locators",
        requested,
        limit: MAX_LOCATORS,
    }
}
