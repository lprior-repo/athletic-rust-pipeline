use crate::bests::key::{PrKey, SurfaceClass, TimingClass, WindClass};
use crate::bests::Measure;
use census_domain::model::{
    AthleteId, EventSpecification, Gender, Mark, PerformanceId, TimingMethod,
};
use census_domain::{JurisdictionBucket, MeetState};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct Population {
    pub marks: usize,
    pub sources: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Conflict {
    pub meet: String,
    pub marks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SelectionResult {
    pub value: i64,
    pub normalized: Option<f64>,
    pub mark: Mark,
    pub place: Option<u16>,
    pub wind_mps: Option<f64>,
    pub timing: Option<TimingMethod>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SelectionMeet {
    pub date: String,
    #[serde(rename = "meet")]
    pub name: String,
    pub meet_id: census_domain::model::MeetId,
    pub meet_state: MeetState,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SelectionSource {
    pub specification: EventSpecification,
    pub result_url: String,
    pub performance_id: PerformanceId,
    pub source_athlete: String,
    pub source_key: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SelectionAthlete {
    #[serde(rename = "athlete")]
    pub name: String,
    pub gender: Gender,
    pub grad_year: i16,
    pub profile_url: Option<String>,
    pub school: Option<String>,
    pub athlete_school: String,
    pub athlete_state: JurisdictionBucket,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SharedSelection {
    pub key: PrKey,
    #[serde(flatten)]
    pub result: SelectionResult,
    #[serde(flatten)]
    pub meet: SelectionMeet,
    #[serde(flatten)]
    pub source: SelectionSource,
    #[serde(flatten)]
    pub athlete: SelectionAthlete,
    pub population: Population,
    pub conflicts: Vec<Conflict>,
}

type OrderKey<'a> = (JurisdictionBucket, &'a str, &'a str, &'a PrKey);

impl SharedSelection {
    pub fn athlete_id(&self) -> &AthleteId {
        &self.key.athlete_id
    }

    pub fn measure(&self) -> Measure {
        self.key.measure
    }

    pub fn event_label(&self) -> String {
        self.key.event_kind.stable_key().into_owned()
    }

    pub fn event_key(&self) -> String {
        self.key.event_kind.stable_key().into_owned()
    }

    pub fn sport(&self) -> &'static str {
        crate::bests::sport_of(&self.key.event_kind)
    }

    pub fn season(&self) -> &'static str {
        match self.key.surface {
            SurfaceClass::Indoor => "Winter",
            SurfaceClass::Outdoor => "Spring",
            SurfaceClass::CrossCountry => "Fall",
            SurfaceClass::Unresolved => "Unresolved",
        }
    }

    pub fn mark_text(&self) -> String {
        super::mark_text(&self.result.mark)
    }

    pub fn unit(&self) -> Option<&'static str> {
        crate::bests::mark_unit(&self.result.mark)
    }

    fn order_key(&self) -> OrderKey<'_> {
        (
            self.athlete.athlete_state,
            self.athlete.athlete_school.as_str(),
            self.athlete.name.as_str(),
            &self.key,
        )
    }
}

pub(crate) fn publish(
    mut rows: Vec<SharedSelection>,
    limit: Option<usize>,
) -> Vec<SharedSelection> {
    rows.sort_unstable_by(|left, right| left.order_key().cmp(&right.order_key()));
    if let Some(limit) = limit {
        rows.truncate(limit);
    }
    rows
}

impl SurfaceClass {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Indoor => "indoor",
            Self::Outdoor => "outdoor",
            Self::CrossCountry => "xc",
            Self::Unresolved => "unresolved",
        }
    }

    pub const fn season_label(self) -> &'static str {
        match self {
            Self::Indoor => "Winter",
            Self::Outdoor => "Spring",
            Self::CrossCountry => "Fall",
            Self::Unresolved => "Unresolved",
        }
    }
}

impl WindClass {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Legal => "legal",
            Self::Assisted => "assisted",
            Self::Unknown => "unknown",
            Self::NotApplicable => "na",
        }
    }
}

impl TimingClass {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Fat => "fat",
            Self::Hand => "hand",
            Self::Unknown => "unknown",
            Self::NonTime => "non_time",
        }
    }
}
