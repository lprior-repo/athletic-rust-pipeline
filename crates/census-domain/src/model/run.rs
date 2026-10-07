use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CensusRun {
    season: SchoolYear,
    revision: u32,
}

impl<'de> Deserialize<'de> for CensusRun {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Repr {
            season: SchoolYear,
            revision: u32,
        }
        let repr = Repr::deserialize(deserializer)?;
        Self::new(repr.season, repr.revision).ok_or_else(|| {
            serde::de::Error::invalid_value(
                serde::de::Unexpected::Unsigned(u64::from(repr.revision)),
                &format!("a run revision of at least {}", Self::MIN_REVISION).as_str(),
            )
        })
    }
}

impl CensusRun {
    pub const MIN_REVISION: u32 = 1;

    pub fn new(season: SchoolYear, revision: u32) -> Option<Self> {
        (revision >= Self::MIN_REVISION).then_some(Self { season, revision })
    }

    pub const fn season(self) -> SchoolYear {
        self.season
    }

    pub const fn revision(self) -> u32 {
        self.revision
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunManifest {
    pub store_identity: String,
    pub run: CensusRun,
    pub cohort: GradYear,
    pub jurisdictions: Vec<UsJurisdiction>,
}
