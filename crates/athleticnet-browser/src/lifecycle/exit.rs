use crate::drain::DrainReport;
use crate::lifecycle::error::BrowserStartupError;

#[derive(Debug)]
pub(crate) struct ActorExit {
    pub(crate) report: DrainReport,
    pub(crate) failure: Option<BrowserStartupError>,
}
