
use census_crawl::applicability::applicable_sources;
use census_crawl::net::Fetcher;
use census_crawl::registry::{AccessClass, SourceDescriptor};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_reconcile::identity::Revision;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserLaneState {
    Configured,
    Absent,
}

impl BrowserLaneState {
    pub fn of(fetcher: &Fetcher) -> Self {
        if fetcher.has_browser_lane() {
            Self::Configured
        } else {
            Self::Absent
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dispatch {
    Wired,
    Unwired,
}

impl Dispatch {
    pub fn of(slug: &str) -> Self {
        if super::jurisdiction::DISPATCHED.contains(&slug) {
            Self::Wired
        } else {
            Self::Unwired
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannedUnit {
    pub slug: &'static str,
    pub access: AccessClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitDisposition {
    Sweep(PlannedUnit),
    Refused(Refusal),
}

const NO_BROWSER_LANE: &str =
    "source needs a browser session and no browser lane is configured: start a lane and re-run";

const NO_JURISDICTION_WALK: &str =
    "no run stage sweeps this source per jurisdiction: its walk is reachable from the CLI only, or \
     it is acquired per meet, team or athlete rather than per state";

impl UnitDisposition {
    pub fn slug(&self) -> &'static str {
        match self {
            Self::Sweep(unit) => unit.slug,
            Self::Refused(refusal) => refusal.slug,
        }
    }
}

pub fn plan(jurisdiction: UsJurisdiction, lane: BrowserLaneState) -> Vec<UnitDisposition> {
    plan_sources(&applicable_sources(jurisdiction), lane)
}

pub fn plan_sources(
    descriptors: &[&'static SourceDescriptor],
    lane: BrowserLaneState,
) -> Vec<UnitDisposition> {
    descriptors
        .iter()
        .map(|descriptor| classify(descriptor, lane))
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Refusal {
    pub slug: &'static str,
    pub access: AccessClass,
    pub reason: &'static str,
}

pub fn sweepable(dispositions: &[UnitDisposition]) -> Vec<PlannedUnit> {
    dispositions
        .iter()
        .filter_map(|disposition| match disposition {
            UnitDisposition::Sweep(unit) => Some(*unit),
            UnitDisposition::Refused(_) => None,
        })
        .collect()
}

pub fn owed(dispositions: &[UnitDisposition]) -> Vec<Refusal> {
    dispositions
        .iter()
        .filter_map(|disposition| match disposition {
            UnitDisposition::Sweep(_) => None,
            UnitDisposition::Refused(refusal) => Some(*refusal),
        })
        .collect()
}

fn classify(descriptor: &'static SourceDescriptor, lane: BrowserLaneState) -> UnitDisposition {
    classify_access(
        descriptor.slug,
        descriptor.access_class(),
        Dispatch::of(descriptor.slug),
        lane,
    )
}

pub(super) fn classify_access(
    slug: &'static str,
    access: AccessClass,
    dispatch: Dispatch,
    lane: BrowserLaneState,
) -> UnitDisposition {
    if dispatch == Dispatch::Unwired {
        return UnitDisposition::Refused(Refusal {
            slug,
            access,
            reason: NO_JURISDICTION_WALK,
        });
    }
    if access == AccessClass::BrowserSession && lane == BrowserLaneState::Absent {
        return UnitDisposition::Refused(Refusal {
            slug,
            access,
            reason: NO_BROWSER_LANE,
        });
    }
    UnitDisposition::Sweep(PlannedUnit { slug, access })
}

pub fn compute_plan_fingerprint(
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    revision: Revision,
    lane: BrowserLaneState,
) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(jurisdiction.code().as_bytes());
    hasher.update(season.short().as_bytes());
    hasher.update(revision.get().to_string().as_bytes());
    hasher.update(match lane {
        BrowserLaneState::Configured => b"C",
        BrowserLaneState::Absent => b"A",
    });
    let bytes = hasher.finalize();
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests;
