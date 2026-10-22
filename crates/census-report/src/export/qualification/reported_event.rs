use census_crawl::milesplit::VerifiedRetainedResultMetadata;
use census_domain::model::{EventKind, Sport};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(crate) struct ReportedEventInterpretation {
    kind: EventKind,
    basis: Basis,
    acquisition_manifest_key: String,
    raw_metadata_manifest_key: String,
    row_locator: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum Basis {
    OwnedDiscReference,
    OwnedShotReference,
    #[serde(rename = "owned_5000m_with_raw_cross_country_context")]
    Owned5000mWithRawCrossCountryContext,
}

impl ReportedEventInterpretation {
    pub(super) fn from_verified(source: &VerifiedRetainedResultMetadata) -> Option<Self> {
        let row = source.result().row();
        let provider = &row.provider;
        let (kind, basis) = match (
            &row.event_kind,
            provider.get("eventCode")?.as_str()?,
            provider.get("eventName")?.as_str()?,
            provider.get("eventDistance")?.as_u64()?,
            source.page().sport,
        ) {
            (EventKind::Discus, "D", "Discus", 0, Some(Sport::OutdoorTrack)) => {
                (EventKind::Discus, Basis::OwnedDiscReference)
            }
            (EventKind::ShotPut, "S", "Shot Put", 0, Some(Sport::OutdoorTrack)) => {
                (EventKind::ShotPut, Basis::OwnedShotReference)
            }
            (EventKind::Track5000m, "5000m", "5000 Meter Run", 5000, Some(Sport::CrossCountry)) => {
                (
                    EventKind::CrossCountry,
                    Basis::Owned5000mWithRawCrossCountryContext,
                )
            }
            _ => return None,
        };
        Some(Self {
            kind,
            basis,
            acquisition_manifest_key: source
                .result()
                .acquisition()
                .reference()
                .manifest_key()
                .into(),
            raw_metadata_manifest_key: source.archive().manifest_key().into(),
            row_locator: row.locator.clone(),
        })
    }

    pub(crate) fn kind(&self) -> &EventKind {
        &self.kind
    }

    pub(crate) fn basis(&self) -> &'static str {
        match self.basis {
            Basis::OwnedDiscReference => "owned_disc_reference",
            Basis::OwnedShotReference => "owned_shot_reference",
            Basis::Owned5000mWithRawCrossCountryContext => {
                "owned_5000m_with_raw_cross_country_context"
            }
        }
    }
}

impl super::ParticipationAssessment {
    pub(crate) fn reported_event(&self) -> Option<&ReportedEventInterpretation> {
        self.reported_event.as_ref()
    }
}

#[derive(Debug, Default)]
pub(super) enum Interpretations {
    #[default]
    Missing,
    Resolved(ReportedEventInterpretation),
    Conflicting,
}

impl Interpretations {
    pub(super) fn retain(&mut self, candidate: Option<ReportedEventInterpretation>) {
        let Some(candidate) = candidate else {
            return;
        };
        match self {
            Self::Missing => *self = Self::Resolved(candidate),
            Self::Resolved(previous) if previous != &candidate => *self = Self::Conflicting,
            Self::Resolved(_) | Self::Conflicting => {}
        }
    }

    pub(super) fn finish(self) -> Option<ReportedEventInterpretation> {
        match self {
            Self::Resolved(value) => Some(value),
            Self::Missing | Self::Conflicting => None,
        }
    }
}
