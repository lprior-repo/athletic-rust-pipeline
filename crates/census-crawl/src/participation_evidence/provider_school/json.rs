use serde::Deserialize;

pub(super) struct MapOnly<T>(pub(super) T);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for MapOnly<T> {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        struct MapVisitor<T>(std::marker::PhantomData<T>);
        impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for MapVisitor<T> {
            type Value = MapOnly<T>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("an object-shaped provider school packet")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                map: M,
            ) -> Result<Self::Value, M::Error> {
                T::deserialize(serde::de::value::MapAccessDeserializer::new(map)).map(MapOnly)
            }
        }
        decoder.deserialize_map(MapVisitor(std::marker::PhantomData))
    }
}

pub(super) struct Capture(pub(super) super::RetainedHtmlCaptureV1);

impl<'de> Deserialize<'de> for Capture {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        struct CaptureVisitor;
        impl<'de> serde::de::Visitor<'de> for CaptureVisitor {
            type Value = Capture;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("an object-shaped retained provider school capture")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> Result<Self::Value, M::Error> {
                let mut fields = serde_json::Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if fields.contains_key(&key) {
                        return Err(serde::de::Error::custom(
                            "duplicate provider school capture field",
                        ));
                    }
                    let value = if key == "provenance" {
                        let MapOnly(provenance) =
                            map.next_value::<MapOnly<super::super::RetainedHttpProvenanceV1>>()?;
                        serde_json::to_value(provenance).map_err(serde::de::Error::custom)?
                    } else {
                        map.next_value::<serde_json::Value>()?
                    };
                    fields.insert(key, value);
                }
                super::RetainedHtmlCaptureV1::deserialize(serde_json::Value::Object(fields))
                    .map(Capture)
                    .map_err(serde::de::Error::custom)
            }
        }
        decoder.deserialize_map(CaptureVisitor)
    }
}
