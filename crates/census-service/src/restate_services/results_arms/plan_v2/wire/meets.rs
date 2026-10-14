use census_domain::model::SourceMeetRef;
use census_domain::UsJurisdiction;
use serde::de::DeserializeSeed;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize, Deserialize)]
#[serde(remote = "SourceMeetRef", deny_unknown_fields)]
struct Meet {
    #[serde(deserialize_with = "super::text::read")]
    id: String,
    #[serde(deserialize_with = "super::text::read")]
    source: String,
    #[serde(deserialize_with = "super::text::read")]
    source_meet_id: String,
    jurisdiction: UsJurisdiction,
    #[serde(deserialize_with = "super::text::read")]
    season: String,
    year: u16,
    #[serde(deserialize_with = "super::text::read")]
    name: String,
    #[serde(deserialize_with = "super::text::optional")]
    date: Option<String>,
    #[serde(deserialize_with = "super::text::read")]
    venue: String,
    #[serde(deserialize_with = "super::text::read")]
    results_url: String,
    #[serde(deserialize_with = "super::text::read")]
    observed_on: String,
}

#[derive(Clone, Copy)]
struct MeetSeed;

impl<'de> DeserializeSeed<'de> for MeetSeed {
    type Value = SourceMeetRef;

    fn deserialize<D: Deserializer<'de>>(self, decoder: D) -> Result<Self::Value, D::Error> {
        Meet::deserialize(decoder)
    }
}

pub(in super::super) fn serialize<S: Serializer>(
    meets: &[SourceMeetRef],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    meets.serialize(serializer)
}

pub(in super::super) fn deserialize<'de, D: Deserializer<'de>>(
    decoder: D,
) -> Result<Vec<SourceMeetRef>, D::Error> {
    decoder.deserialize_seq(super::bounded::Sequence {
        seed: MeetSeed,
        limit: 65_536,
    })
}

pub(in super::super) fn strings(meet: &SourceMeetRef) -> [&str; 9] {
    [
        &meet.id,
        &meet.source,
        &meet.source_meet_id,
        &meet.season,
        &meet.name,
        meet.date.as_deref().map_or("", core::convert::identity),
        &meet.venue,
        &meet.results_url,
        &meet.observed_on,
    ]
}

pub(in super::super) fn capacities(meet: &SourceMeetRef) -> [usize; 9] {
    [
        meet.id.capacity(),
        meet.source.capacity(),
        meet.source_meet_id.capacity(),
        meet.season.capacity(),
        meet.name.capacity(),
        meet.date.as_ref().map_or(0, String::capacity),
        meet.venue.capacity(),
        meet.results_url.capacity(),
        meet.observed_on.capacity(),
    ]
}
