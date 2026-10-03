use super::{match_owner, refuse, Owner};
use crate::milesplit::wire::TeamRef;
use crate::CrawlResult;
use serde::de::{SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::borrow::Cow;
use std::fmt;
use url::Url;

#[derive(Default, Clone, Copy)]
enum Kind {
    WebPage,
    Team,
    #[default]
    Other,
}

impl Kind {
    fn parse(value: &str) -> Self {
        match value {
            "WebPage" | "https://schema.org/WebPage" | "http://schema.org/WebPage" => Self::WebPage,
            "School"
            | "SportsTeam"
            | "https://schema.org/School"
            | "https://schema.org/SportsTeam"
            | "http://schema.org/School"
            | "http://schema.org/SportsTeam" => Self::Team,
            _ => Self::Other,
        }
    }
}

impl<'de> Deserialize<'de> for Kind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(KindVisitor)
    }
}

struct KindVisitor;

impl<'de> Visitor<'de> for KindVisitor {
    type Value = Kind;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a published entity type or list of entity types")
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Kind, E> {
        Ok(Kind::parse(value))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Kind, A::Error> {
        std::iter::from_fn(|| sequence.next_element::<&'de str>().transpose()).try_fold(
            Kind::Other,
            |current, next| {
                Ok(match (current, Kind::parse(next?)) {
                    (Kind::WebPage, _) | (_, Kind::WebPage) => Kind::WebPage,
                    (Kind::Team, _) | (_, Kind::Team) => Kind::Team,
                    _ => Kind::Other,
                })
            },
        )
    }
}

#[derive(Deserialize)]
struct Document<'a> {
    #[serde(rename = "@type", default)]
    kind: Kind,
    #[serde(borrow)]
    url: Option<Cow<'a, str>>,
    #[serde(rename = "@id", borrow)]
    id: Option<Cow<'a, str>>,
}

pub(super) fn document(
    body: &str,
    base: &Url,
    team: &TeamRef,
    expected: Owner,
) -> CrawlResult<bool> {
    if !body.trim_start().starts_with('{') {
        return Ok(false);
    }
    let document: Document<'_> = serde_json::from_str(body)
        .map_err(|_| refuse(team, "malformed published roster JSON metadata"))?;
    if matches!(document.kind, Kind::Other) {
        return Ok(false);
    }
    document
        .url
        .as_deref()
        .into_iter()
        .chain(document.id.as_deref())
        .try_fold(false, |found, raw| {
            if raw.trim().is_empty() || raw.trim().starts_with(['?', '#']) {
                return Ok(found);
            }
            let resolved = base
                .join(raw)
                .map_err(|_| refuse(team, "malformed published roster JSON URL"))?;
            let team_path =
                resolved.path_segments().and_then(|mut path| path.next()) == Some("teams");
            if matches!(document.kind, Kind::Team) && !team_path {
                return Ok(found);
            }
            match_owner(resolved.as_str(), team, expected, false)?;
            Ok(true)
        })
}
