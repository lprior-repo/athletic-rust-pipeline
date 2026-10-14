use census_crawl::milesplit::{ResultSetOptions, ResultSetRequest};
use census_domain::UsJurisdiction;
use serde::de::DeserializeSeed;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize, Deserialize)]
#[serde(remote = "ResultSetOptions", deny_unknown_fields)]
struct Options {
    #[serde(deserialize_with = "requests", serialize_with = "write_requests")]
    urls: Vec<ResultSetRequest>,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "ResultSetRequest", deny_unknown_fields)]
struct Request {
    #[serde(deserialize_with = "super::text::read")]
    url: String,
    jurisdiction: UsJurisdiction,
}

#[derive(Clone, Copy)]
struct RequestSeed;

impl<'de> DeserializeSeed<'de> for RequestSeed {
    type Value = ResultSetRequest;

    fn deserialize<D: Deserializer<'de>>(self, decoder: D) -> Result<Self::Value, D::Error> {
        Request::deserialize(decoder)
    }
}

fn requests<'de, D: Deserializer<'de>>(decoder: D) -> Result<Vec<ResultSetRequest>, D::Error> {
    decoder.deserialize_seq(super::bounded::Sequence {
        seed: RequestSeed,
        limit: 4096,
    })
}

fn write_requests<S: Serializer>(
    requests: &[ResultSetRequest],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeSeq;
    #[derive(Serialize)]
    struct Ref<'a>(#[serde(with = "Request")] &'a ResultSetRequest);
    let mut sequence = serializer.serialize_seq(Some(requests.len()))?;
    for request in requests {
        sequence.serialize_element(&Ref(request))?;
    }
    sequence.end()
}

pub(in crate::restate_services) fn serialize<S: Serializer>(
    value: &ResultSetOptions,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    Options::serialize(value, serializer)
}

pub(in crate::restate_services) fn deserialize<'de, D: Deserializer<'de>>(
    decoder: D,
) -> Result<ResultSetOptions, D::Error> {
    Options::deserialize(decoder)
}
