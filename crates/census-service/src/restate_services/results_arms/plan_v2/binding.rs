use super::ResultsPlanError;
use census_domain::model::{
    AcquisitionDigest, CaptureCacheIntent, CensusRun, HistoryWindow, ResultsCaptureScopeV1,
    SelectedHistoryV1, SourceAdmissionV1, StoreIdentity,
};
use census_domain::UsJurisdiction;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, PartialEq, Eq)]
pub(in crate::restate_services) struct ResultsPlanBinding {
    identity: StoreIdentity,
    scope: ResultsCaptureScopeV1,
    observed_on: String,
}

#[derive(Serialize)]
struct Fields<'a> {
    store_identity: &'a StoreIdentity,
    owner_key: &'a str,
    jurisdiction: UsJurisdiction,
    run: CensusRun,
    run_manifest_digest: &'a AcquisitionDigest,
    history: HistoryWindow,
    historical_year: u16,
    source_admission: &'a SourceAdmissionV1,
    cache_intent: CaptureCacheIntent,
    observed_on: &'a str,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    store_identity: StoreIdentity,
    #[serde(deserialize_with = "super::wire::read_text")]
    owner_key: String,
    jurisdiction: UsJurisdiction,
    run: CensusRun,
    run_manifest_digest: AcquisitionDigest,
    #[serde(deserialize_with = "history")]
    history: HistoryWindow,
    historical_year: u16,
    source_admission: SourceAdmissionV1,
    cache_intent: CaptureCacheIntent,
    #[serde(deserialize_with = "super::wire::read_text")]
    observed_on: String,
}

impl ResultsPlanBinding {
    pub(in crate::restate_services) fn new(
        identity: StoreIdentity,
        scope: ResultsCaptureScopeV1,
        observed_on: String,
    ) -> Result<Self, ResultsPlanError> {
        let date = chrono::NaiveDate::parse_from_str(&observed_on, "%Y-%m-%d")
            .map_err(|_| ResultsPlanError::Invalid("observation date"))?;
        if date.to_string() != observed_on {
            return Err(ResultsPlanError::Invalid("observation date"));
        }
        Ok(Self {
            identity,
            scope,
            observed_on,
        })
    }

    pub(in crate::restate_services) fn identity(&self) -> &StoreIdentity {
        &self.identity
    }
    pub(in crate::restate_services) fn scope(&self) -> &ResultsCaptureScopeV1 {
        &self.scope
    }

    pub(in crate::restate_services) fn require_owner(
        &self,
        owner: &census_store::ResultsCaptureOwnerV1,
    ) -> Result<(), ResultsPlanError> {
        if owner.protocol() != census_domain::model::CaptureProtocolVersion::V2
            || owner.store_identity() != &self.identity
            || owner.scope() != &self.scope
        {
            return Err(ResultsPlanError::Invalid("actual owner binding"));
        }
        Ok(())
    }

    pub(in crate::restate_services) fn require_stage(
        &self,
        scope: crate::restate_services::history_stage::HistoricalStageScope,
    ) -> Result<(), ResultsPlanError> {
        let observed_on = chrono::NaiveDate::parse_from_str(&self.observed_on, "%Y-%m-%d")
            .map_err(|_| ResultsPlanError::Invalid("observation date"))?;
        if self.scope.jurisdiction() != scope.jurisdiction
            || self.scope.historical_year() != scope.year
            || self.scope.history() != scope.window
            || observed_on != scope.observed_on
            || (self.scope.cache_intent() == CaptureCacheIntent::AcquireNew) != scope.refresh
        {
            return Err(ResultsPlanError::Invalid("actual stage binding"));
        }
        Ok(())
    }
}

impl Serialize for ResultsPlanBinding {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        Fields {
            store_identity: &self.identity,
            owner_key: self.scope.owner_key(),
            jurisdiction: self.scope.jurisdiction(),
            run: self.scope.run(),
            run_manifest_digest: self.scope.run_manifest_digest(),
            history: self.scope.history(),
            historical_year: self.scope.historical_year(),
            source_admission: self.scope.source_admission(),
            cache_intent: self.scope.cache_intent(),
            observed_on: &self.observed_on,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ResultsPlanBinding {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let wire = Wire::deserialize(decoder)?;
        let history =
            SelectedHistoryV1::new(wire.history, wire.historical_year).map_err(D::Error::custom)?;
        let scope = ResultsCaptureScopeV1::new(
            &wire.owner_key,
            wire.jurisdiction,
            wire.run,
            wire.run_manifest_digest,
            history,
            wire.source_admission,
            wire.cache_intent,
        )
        .map_err(D::Error::custom)?;
        Self::new(wire.store_identity, scope, wire.observed_on).map_err(D::Error::custom)
    }
}

fn history<'de, D: Deserializer<'de>>(decoder: D) -> Result<HistoryWindow, D::Error> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct History {
        first_calendar_year: u16,
        last_calendar_year: u16,
        #[serde(deserialize_with = "super::wire::read_text")]
        as_of: String,
    }
    let value = History::deserialize(decoder)?;
    HistoryWindow::new(
        value.first_calendar_year,
        value.last_calendar_year,
        &value.as_of,
    )
    .map_err(serde::de::Error::custom)
}
