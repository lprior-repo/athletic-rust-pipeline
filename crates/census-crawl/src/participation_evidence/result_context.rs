use super::{bound, invalid, markup, MAX_BODY};
use crate::milesplit::VerifiedRetainedResultMetadata;
use census_store::{StoreError, StoreResult};
use html5gum::{DefaultEmitter, Token, Tokenizer};
use serde::Deserialize;

pub fn retained_result_venue(source: &VerifiedRetainedResultMetadata) -> StoreResult<String> {
    let capture = source.capture();
    bound(
        capture.body.len(),
        MAX_BODY,
        "participation result context body bytes",
    )?;
    let html =
        std::str::from_utf8(&capture.body).map_err(|_| invalid("result context is not UTF-8"))?;
    read(
        html,
        &capture.url,
        &source.page().meet.name,
        &source.page().meet.date,
    )
}

fn read(html: &str, owner: &str, name: &str, date: &str) -> StoreResult<String> {
    markup::canonical(html, owner)?;
    let scripts = markup::elements(html, b"script", None)?;
    let selected = scripts.iter().try_fold(None, |selected, script| {
        if !script
            .attribute("type")
            .is_some_and(|value| value.eq_ignore_ascii_case("application/ld+json"))
        {
            return Ok(selected);
        }
        let text = script_text(script.html)?;
        let MapOnly(value): MapOnly<Event> = serde_json::from_str(&text).map_err(super::json)?;
        if value.graph.is_some() {
            return Err(invalid("unsupported result context JSON-LD graph"));
        }
        let kind = value
            .kind
            .as_deref()
            .ok_or_else(|| invalid("unsupported result context type"))?;
        if !matches!(
            kind,
            "SportsEvent" | "https://schema.org/SportsEvent" | "http://schema.org/SportsEvent"
        ) {
            return Ok(selected);
        }
        if selected.is_some() {
            return Err(invalid("result context SportsEvent is not unique"));
        }
        let venue = venue(&value, owner, name, date)?;
        Ok(Some(venue))
    })?;
    selected.ok_or_else(|| invalid("owned result venue context absent"))
}

fn venue(value: &Event, owner: &str, name: &str, date: &str) -> StoreResult<String> {
    let location = &value
        .location
        .as_ref()
        .ok_or_else(|| invalid("owned result location absent"))?
        .0;
    if value.url.as_deref() != Some(owner)
        || value.id.as_ref().is_some_and(|id| id != owner)
        || value.name.as_deref() != Some(name)
        || value.start_date.as_deref() != Some(date)
        || value.sport.as_deref() != Some("Outdoor Track & Field")
        || location
            .address
            .as_ref()
            .and_then(|address| address.0.region.as_deref())
            != Some("MI")
    {
        return Err(invalid(
            "result venue context differs from verified metadata",
        ));
    }
    let venue = location
        .name
        .as_deref()
        .ok_or_else(|| invalid("owned result venue name absent"))?;
    bound(venue.len(), 256, "participation result venue bytes")?;
    if venue.is_empty()
        || venue != venue.trim()
        || venue.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(invalid("owned result venue name unsupported"));
    }
    Ok(venue.into())
}

#[derive(Deserialize)]
struct Event {
    #[serde(rename = "@type")]
    kind: Option<String>,
    #[serde(rename = "@graph")]
    graph: Option<serde::de::IgnoredAny>,
    url: Option<String>,
    #[serde(rename = "@id")]
    id: Option<String>,
    name: Option<String>,
    #[serde(rename = "startDate")]
    start_date: Option<String>,
    sport: Option<String>,
    location: Option<MapOnly<Location>>,
}

#[derive(Deserialize)]
struct Location {
    name: Option<String>,
    address: Option<MapOnly<Address>>,
}

#[derive(Deserialize)]
struct Address {
    #[serde(rename = "addressRegion")]
    region: Option<String>,
}

struct MapOnly<T>(T);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for MapOnly<T> {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        struct MapVisitor<T>(std::marker::PhantomData<T>);
        impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for MapVisitor<T> {
            type Value = MapOnly<T>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("an object-shaped result context")
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

fn script_text(html: &str) -> StoreResult<String> {
    let mut emitter = DefaultEmitter::default();
    emitter.naively_switch_states(true);
    Tokenizer::new_with_emitter(html, emitter)
        .take(4097)
        .enumerate()
        .try_fold(String::new(), |mut body, (index, token)| {
            bound(
                index.checked_add(1).ok_or(StoreError::CounterOverflow)?,
                4096,
                "participation result context token count",
            )?;
            if let Token::String(value) =
                token.map_err(|_| invalid("result context token failure"))?
            {
                let text = std::str::from_utf8(&value)
                    .map_err(|_| invalid("result context JSON encoding"))?;
                bound(
                    body.len()
                        .checked_add(text.len())
                        .ok_or(StoreError::CounterOverflow)?,
                    64 * 1024,
                    "participation result context JSON bytes",
                )?;
                body.try_reserve(text.len())
                    .map_err(|_| invalid("result context JSON allocation refused"))?;
                body.push_str(text);
            }
            Ok(body)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    type TestResult = Result<(), Box<dyn std::error::Error>>;
    const OWNER: &str = "https://mi.milesplit.com/meets/1-synthetic/results/2/raw";

    fn document(venue: &str) -> String {
        format!(
            r#"<link rel="canonical" href="{OWNER}"><script type="application/ld+json">{{"@type":"SportsEvent","url":"{OWNER}","name":"Synthetic Meet","startDate":"2026-03-27","sport":"Outdoor Track & Field","location":{{"name":"{venue}","address":{{"addressRegion":"MI"}}}}}}</script>"#
        )
    }

    #[test]
    fn venue_requires_same_owned_unique_metadata_context() -> TestResult {
        let html = document("Synthetic Venue");
        assert_eq!(
            read(&html, OWNER, "Synthetic Meet", "2026-03-27")?,
            "Synthetic Venue"
        );
        for changed in [
            html.replace("Synthetic Meet", "Foreign Meet"),
            html.replace("2026-03-27", "2026-03-28"),
            html.replace("Synthetic Venue", ""),
            format!("{html}{}", document("Other Venue")),
        ] {
            assert!(read(&changed, OWNER, "Synthetic Meet", "2026-03-27").is_err());
        }
        for changed in [
            html.replace("\"location\":{", "\"location\":{},\"location\":{"),
            html.replace(
                "\"name\":\"Synthetic Venue\"",
                "\"name\":\"Other Venue\",\"name\":\"Synthetic Venue\"",
            ),
        ] {
            assert!(read(&changed, OWNER, "Synthetic Meet", "2026-03-27").is_err());
        }
        let array = format!(
            r#"["SportsEvent",null,"{OWNER}",null,"Synthetic Meet","2026-03-27","Outdoor Track & Field",{{"name":"Synthetic Venue","address":{{"addressRegion":"MI"}}}}]"#
        );
        let array_document = format!(
            r#"<link rel="canonical" href="{OWNER}"><script type="application/ld+json">{array}</script>"#
        );
        for changed in [
            array_document,
            html.replace(
                r#"{"name":"Synthetic Venue","address":{"addressRegion":"MI"}}"#,
                r#"["Synthetic Venue",{"addressRegion":"MI"}]"#,
            ),
            html.replace(r#"{"addressRegion":"MI"}"#, r#"["MI"]"#),
        ] {
            assert!(read(&changed, OWNER, "Synthetic Meet", "2026-03-27").is_err());
        }
        for attribute in ["hidden", "inert"] {
            assert!(read(
                &format!("<div {attribute}>{html}</div>"),
                OWNER,
                "Synthetic Meet",
                "2026-03-27"
            )
            .is_err());
        }
        Ok(())
    }
}
