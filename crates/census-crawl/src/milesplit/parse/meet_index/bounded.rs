use super::{
    legacy_fields, meet_result_files_regex, DD_RESULTS_PAGE_REGEX, INLINE_RESULTS_ANCHOR,
    LEGACY_OPTION_REGEX,
};
use crate::milesplit::results::pages::bounded::{resource, Allocation};
use crate::milesplit::wire::MeetResultFile;
use crate::{CrawlError, CrawlResult};
use serde::de::{DeserializeSeed, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;

pub(in crate::milesplit) fn parse_bounded(
    url: &str,
    html: &str,
    maximum: usize,
    allocation: &mut Allocation,
) -> CrawlResult<Vec<MeetResultFile>> {
    allocation.reserve(8192)?;
    let regex = meet_result_files_regex()?;
    if let Some(captured) = regex.captures(html) {
        let literal = captured.get(1).map_or("", |value| value.as_str());
        allocation.reserve(
            literal
                .len()
                .checked_mul(4)
                .ok_or_else(|| resource("meet-page allocation", usize::MAX, usize::MAX))?,
        )?;
        let mut refused = None;
        let mut decoder = serde_json::Deserializer::from_str(literal);
        let parsed = Files {
            maximum,
            allocation,
            refused: &mut refused,
        }
        .deserialize(&mut decoder);
        let files = match parsed {
            Ok(files) => files,
            Err(source) => {
                return Err(match refused {
                    Some(error) => error,
                    None => CrawlError::Decode {
                        url: "meet results page".to_string(),
                        source,
                    },
                })
            }
        };
        decoder.end().map_err(|source| CrawlError::Decode {
            url: "meet results page".to_string(),
            source,
        })?;
        return Ok(files);
    }
    if let (Ok(select), Ok(options)) =
        (DD_RESULTS_PAGE_REGEX.as_ref(), LEGACY_OPTION_REGEX.as_ref())
    {
        if let Some(select) = select.captures(html).and_then(|value| value.get(1)) {
            let mut files = Vec::new();
            let mut matches = options.captures_iter(select.as_str());
            loop {
                allocation.reserve(512)?;
                let Some(option) = matches.next() else {
                    break;
                };
                if let Some((id, _)) = option
                    .get(0)
                    .and_then(|value| legacy_fields(value.as_str()))
                {
                    allocation.push(
                        &mut files,
                        MeetResultFile {
                            id,
                            name: String::new(),
                            is_meet_pro: 0,
                            inline: false,
                        },
                        maximum,
                    )?;
                }
            }
            return Ok(files);
        }
    }
    if html.contains(INLINE_RESULTS_ANCHOR) {
        let mut files = Vec::new();
        allocation.push(
            &mut files,
            MeetResultFile {
                id: 0,
                name: String::new(),
                is_meet_pro: 0,
                inline: true,
            },
            maximum,
        )?;
        return Ok(files);
    }
    allocation.reserve(
        url.len()
            .checked_add(96)
            .ok_or_else(|| resource("meet-page allocation", usize::MAX, usize::MAX))?,
    )?;
    Err(CrawlError::Schema {
        url: url.to_string(),
        detail: "no meetResultFiles literal, ddResultsPage select or meetResultsBody block"
            .to_string(),
    })
}

struct Files<'a> {
    maximum: usize,
    allocation: &'a mut Allocation,
    refused: &'a mut Option<CrawlError>,
}

impl<'de> DeserializeSeed<'de> for Files<'_> {
    type Value = Vec<MeetResultFile>;
    fn deserialize<D: Deserializer<'de>>(self, decoder: D) -> Result<Self::Value, D::Error> {
        decoder.deserialize_seq(self)
    }
}

impl<'de> Visitor<'de> for Files<'_> {
    type Value = Vec<MeetResultFile>;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("bounded meet result files")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
        let mut files = Vec::new();
        while let Some(file) = access.next_element_seed(File {
            allowed: files.len() < self.maximum,
            maximum: self.maximum,
            refused: self.refused,
        })? {
            if let Err(error) = self.allocation.push(&mut files, file, self.maximum) {
                *self.refused = Some(error);
                return Err(serde::de::Error::custom(
                    "bounded discovery allocation refused",
                ));
            }
        }
        Ok(files)
    }
}

struct File<'a> {
    allowed: bool,
    maximum: usize,
    refused: &'a mut Option<CrawlError>,
}

impl<'de> DeserializeSeed<'de> for File<'_> {
    type Value = MeetResultFile;
    fn deserialize<D: Deserializer<'de>>(self, decoder: D) -> Result<Self::Value, D::Error> {
        if !self.allowed {
            *self.refused = Some(resource(
                "meet-page files",
                self.maximum.saturating_add(1),
                self.maximum,
            ));
            return Err(serde::de::Error::custom(
                "meet-page file limit exceeded before growth",
            ));
        }
        let file = Projection::deserialize(decoder)?;
        Ok(MeetResultFile {
            id: file.id,
            name: String::new(),
            is_meet_pro: file.is_meet_pro,
            inline: file.inline,
        })
    }
}

#[derive(Deserialize)]
struct Projection {
    id: i64,
    #[serde(default)]
    #[serde(rename = "name")]
    _name: IgnoredName,
    #[serde(default, rename = "isMeetPro")]
    is_meet_pro: i64,
    #[serde(default)]
    inline: bool,
}

#[derive(Default)]
struct IgnoredName;

impl<'de> Deserialize<'de> for IgnoredName {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        decoder.deserialize_str(NameVisitor)
    }
}

struct NameVisitor;

impl Visitor<'_> for NameVisitor {
    type Value = IgnoredName;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("result file name")
    }
    fn visit_str<E: serde::de::Error>(self, _: &str) -> Result<Self::Value, E> {
        Ok(IgnoredName)
    }
}
