use census_domain::model::{
    AcquisitionAttemptId, AcquisitionDispatchId, NormalizedObservedHttpV1, ObservedHttpHopV1,
    ObservedHttpRequestV1, ObservedHttpResponseV1,
};
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Serialize, Serializer};

pub(super) struct Observed<'a>(pub(super) &'a NormalizedObservedHttpV1);

impl Serialize for Observed<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let value = self.0;
        let mut map = serializer.serialize_map(Some(6))?;
        map.serialize_entry("body_bytes", &value.body_bytes())?;
        map.serialize_entry("body_digest", value.body_digest())?;
        map.serialize_entry("body_encoding", "identity")?;
        map.serialize_entry("completed_at", value.completed_at())?;
        map.serialize_entry("hops", &Hops(value.hops()))?;
        map.serialize_entry("recipe", "reqwest_normalized_observed_http_v1")?;
        map.end()
    }
}

struct Hops<'a>(&'a [ObservedHttpHopV1]);

impl Serialize for Hops<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for hop in self.0 {
            sequence.serialize_element(&Hop(hop))?;
        }
        sequence.end()
    }
}

struct Hop<'a>(&'a ObservedHttpHopV1);

impl Serialize for Hop<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("dispatch", &Dispatch(self.0.dispatch()))?;
        map.serialize_entry("request", &Request(self.0.request()))?;
        map.serialize_entry("response", &Response(self.0.response()))?;
        map.end()
    }
}

struct Dispatch<'a>(&'a AcquisitionDispatchId);

impl Serialize for Dispatch<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("attempt", &Attempt(self.0.attempt()))?;
        map.serialize_entry("hop", &self.0.hop())?;
        map.end()
    }
}

struct Attempt<'a>(&'a AcquisitionAttemptId);

impl Serialize for Attempt<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("ordinal", &self.0.ordinal())?;
        map.serialize_entry("store_identity", self.0.store_identity())?;
        map.serialize_entry("work_id", self.0.work_id())?;
        map.end()
    }
}

struct Request<'a>(&'a ObservedHttpRequestV1);

impl Serialize for Request<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("headers", self.0.headers())?;
        map.serialize_entry("started_at", self.0.started_at())?;
        map.serialize_entry("target_url", self.0.target_url())?;
        map.end()
    }
}

struct Response<'a>(&'a ObservedHttpResponseV1);

impl Serialize for Response<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("headers", self.0.headers())?;
        map.serialize_entry("protocol", &self.0.protocol())?;
        map.serialize_entry("received_at", self.0.received_at())?;
        map.serialize_entry("response_url", self.0.response_url())?;
        map.serialize_entry("status", &self.0.status())?;
        map.end()
    }
}
