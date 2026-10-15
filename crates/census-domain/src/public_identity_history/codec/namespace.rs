use std::fmt;

use crate::model::SourceNamespace;
use serde::de::{DeserializeSeed, EnumAccess, Error, MapAccess, VariantAccess, Visitor};
use serde::{Deserialize, Deserializer};

use super::atoms::TextNode;
use super::containers::{copy_text, Meter, Node, NodeSeed};

pub(super) struct NamespaceNode;

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum NamespaceVariant {
    MilesplitAthlete,
    TfrrsAthlete,
    DirectAthleticsAthlete,
    AssociationAthlete,
    TimerAthlete,
    AthleticNet,
}

impl Node for NamespaceNode {
    type Value = SourceNamespace;

    fn decode<'de, D: Deserializer<'de>>(
        meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_enum(
            "SourceNamespace",
            &[
                "milesplit_athlete",
                "tfrrs_athlete",
                "direct_athletics_athlete",
                "association_athlete",
                "timer_athlete",
                "athletic_net",
            ],
            NodeSeed::<Self>::new(meter),
        )
    }
}

impl<'de> Visitor<'de> for NodeSeed<'_, NamespaceNode> {
    type Value = SourceNamespace;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an athlete source namespace")
    }

    fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> Result<Self::Value, A::Error> {
        let (tag, variant) = data.variant::<NamespaceVariant>()?;
        match tag {
            NamespaceVariant::MilesplitAthlete => variant
                .unit_variant()
                .map(|()| SourceNamespace::MilesplitAthlete),
            NamespaceVariant::TfrrsAthlete => variant
                .unit_variant()
                .map(|()| SourceNamespace::TfrrsAthlete),
            NamespaceVariant::DirectAthleticsAthlete => variant
                .unit_variant()
                .map(|()| SourceNamespace::DirectAthleticsAthlete),
            NamespaceVariant::AssociationAthlete => variant
                .newtype_variant_seed(PayloadSeed {
                    meter: self.meter,
                    field: PayloadField::Association,
                })
                .map(|association| SourceNamespace::AssociationAthlete { association }),
            NamespaceVariant::TimerAthlete => variant
                .newtype_variant_seed(PayloadSeed {
                    meter: self.meter,
                    field: PayloadField::Provider,
                })
                .map(|provider| SourceNamespace::TimerAthlete { provider }),
            NamespaceVariant::AthleticNet => variant
                .newtype_variant_seed(PayloadSeed {
                    meter: self.meter,
                    field: PayloadField::Kind,
                })
                .map(|kind| SourceNamespace::AthleticNet { kind }),
        }
    }
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(field_identifier, rename_all = "snake_case")]
enum PayloadField {
    Association,
    Provider,
    Kind,
}

impl PayloadField {
    fn name(self) -> &'static str {
        match self {
            Self::Association => "association",
            Self::Provider => "provider",
            Self::Kind => "kind",
        }
    }
}

struct PayloadSeed<'a> {
    meter: &'a mut Meter,
    field: PayloadField,
}

impl<'de> DeserializeSeed<'de> for PayloadSeed<'_> {
    type Value = String;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for PayloadSeed<'_> {
    type Value = String;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("one bounded source namespace field")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let field = map
            .next_key::<PayloadField>()?
            .ok_or_else(|| A::Error::missing_field(self.field.name()))?;
        if field != self.field {
            return Err(A::Error::custom("unexpected source namespace field"));
        }
        let value = match self.field {
            PayloadField::Kind => map.next_value_seed(AthleteKindSeed)?,
            PayloadField::Association | PayloadField::Provider => {
                map.next_value_seed(NodeSeed::<TextNode>::new(self.meter))?
            }
        };
        if map.next_key::<PayloadField>()?.is_some() {
            return Err(A::Error::custom(
                "duplicate or unexpected source namespace field",
            ));
        }
        Ok(value)
    }
}

struct AthleteKindSeed;

impl<'de> DeserializeSeed<'de> for AthleteKindSeed {
    type Value = String;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_str(self)
    }
}

impl Visitor<'_> for AthleteKindSeed {
    type Value = String;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("athlete")
    }

    fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
        if value != "athlete" {
            return Err(E::custom("invalid athlete source namespace"));
        }
        copy_text(value)
    }
}
