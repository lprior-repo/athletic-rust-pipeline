use super::{filesystem, invariant, Generation, Manifest, ResultSetRequest, INPUT_PHASE};
use crate::{CrawlError, CrawlResult};
use serde::de::{DeserializeSeed, IgnoredAny, SeqAccess, Visitor};
use std::fmt;
use std::ops::Range;
use std::path::Path;

mod requests;

pub(in crate::milesplit::results) fn range(
    locator: &str,
    mut visit: impl FnMut(usize, ResultSetRequest) -> CrawlResult<()>,
) -> CrawlResult<()> {
    let reference = Reference::parse(locator)?;
    let path = reference
        .uri
        .to_file_path()
        .map_err(|()| invariant("request manifest is not a local file"))?;
    let manifest = manifest(&path)?;
    reference.validate(&manifest)?;
    let parent = path
        .parent()
        .ok_or_else(|| invariant("request manifest has no archive directory"))?;
    let input = parent.join(&manifest.input_file);
    filesystem::verify(&input, &manifest.input_digest, manifest.bytes)?;
    let file = filesystem::open(&input)?;
    let mut outcome = requests::Outcome::Running;
    let mut decoder = serde_json::Deserializer::from_reader(file);
    let result = Input {
        manifest: &manifest,
        range: reference.range,
        visit: &mut visit,
        outcome: &mut outcome,
    }
    .deserialize(&mut decoder);
    if let requests::Outcome::Failed(error) = outcome {
        return Err(error);
    }
    result.map_err(decode_error)?;
    decoder.end().map_err(decode_error)
}

fn manifest(path: &Path) -> CrawlResult<Manifest> {
    let metadata = std::fs::metadata(path).map_err(|source| filesystem::io_error(path, source))?;
    super::limit(
        "request manifest bytes",
        usize::try_from(metadata.len())
            .map_err(|_| invariant("request manifest length exceeds platform range"))?,
        16 * 1024,
    )?;
    if !metadata.is_file() || !metadata.permissions().readonly() {
        return Err(invariant("request manifest is not immutable"));
    }
    let file = filesystem::open(path)?;
    serde_json::from_reader(file).map_err(decode_error)
}

struct Reference {
    uri: url::Url,
    range: Range<usize>,
}

impl Reference {
    fn parse(locator: &str) -> CrawlResult<Self> {
        super::limit("request replay locator bytes", locator.len(), 16 * 1024)?;
        let uri = url::Url::parse(locator)
            .map_err(|_| invariant("request replay locator is not a URL"))?;
        if uri.scheme() != "file" || uri.query().is_some() {
            return Err(invariant(
                "request replay locator is not an immutable file reference",
            ));
        }
        let mut result = Self { uri, range: 0..0 };
        validate_fragment(&result.uri)?;
        let (start, end) = result
            .parameter("requests")?
            .split_once("..")
            .ok_or_else(|| invariant("request replay range is malformed"))?;
        let start = start
            .parse::<usize>()
            .map_err(|_| invariant("request replay start is not an ordinal"))?;
        let end = end
            .parse::<usize>()
            .map_err(|_| invariant("request replay end is not an ordinal"))?;
        if start > end {
            return Err(invariant("request replay range is reversed"));
        }
        result.range = start..end;
        Ok(result)
    }

    fn parameter(&self, name: &str) -> CrawlResult<&str> {
        let fragment = self
            .uri
            .fragment()
            .ok_or_else(|| invariant("request replay locator has no input binding"))?;
        let mut fields = fragment
            .split(';')
            .filter(|field| field.split_once('=').is_some_and(|(key, _)| key == name));
        let value = fields
            .next()
            .and_then(|field| field.split_once('='))
            .map(|(_, value)| value)
            .ok_or_else(|| invariant("request replay locator has an incomplete input binding"))?;
        if fields.next().is_some() {
            return Err(invariant("request replay locator repeats an input binding"));
        }
        Ok(value)
    }

    fn validate(&self, manifest: &Manifest) -> CrawlResult<()> {
        super::limit(
            "request replay count",
            manifest.request_count,
            super::MAX_WORK / 4,
        )?;
        if !is_digest(&manifest.input_digest) || !is_digest(&manifest.generation_digest) {
            return Err(invariant("request replay digest is not canonical SHA256"));
        }
        if self.parameter("input_sha256")? != manifest.input_digest
            || self.parameter("generation_sha256")? != manifest.generation_digest
        {
            return Err(invariant(
                "request replay locator does not bind committed input and generation",
            ));
        }
        if manifest.input_file != format!("{}.json", manifest.input_digest)
            || self.range.end > manifest.request_count
        {
            return Err(invariant(
                "request replay file or range does not match committed input",
            ));
        }
        let pending = self
            .parameter("pending")?
            .parse::<usize>()
            .map_err(|_| invariant("request replay pending count is not an ordinal"))?;
        let available = self
            .range
            .end
            .checked_sub(self.range.start)
            .ok_or_else(|| invariant("request replay range is reversed"))?;
        if pending > available {
            return Err(invariant(
                "request replay pending count exceeds original ordinal range",
            ));
        }
        Ok(())
    }
}

struct Input<'a, F> {
    manifest: &'a Manifest,
    range: Range<usize>,
    visit: &'a mut F,
    outcome: &'a mut requests::Outcome,
}

impl<'de, F: FnMut(usize, ResultSetRequest) -> CrawlResult<()>> DeserializeSeed<'de>
    for Input<'_, F>
{
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<(), D::Error> {
        decoder.deserialize_tuple(2, self)
    }
}

impl<'de, F: FnMut(usize, ResultSetRequest) -> CrawlResult<()>> Visitor<'de> for Input<'_, F> {
    type Value = ();
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a generation and immutable request sequence")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<(), A::Error> {
        let generation = sequence
            .next_element::<Generation>()?
            .ok_or_else(|| serde::de::Error::custom("request generation is missing"))?;
        if generation.0 != INPUT_PHASE
            || filesystem::digest(&generation).map_err(serde::de::Error::custom)?
                != self.manifest.generation_digest
        {
            return Err(serde::de::Error::custom(
                "request generation differs from committed manifest",
            ));
        }
        let rows = requests::Requests {
            range: self.range,
            expected: self.manifest.request_count,
            visit: self.visit,
            outcome: self.outcome,
        };
        sequence
            .next_element_seed(rows)?
            .ok_or_else(|| serde::de::Error::custom("immutable request sequence is missing"))?;
        if sequence.next_element::<IgnoredAny>()?.is_some() {
            return Err(serde::de::Error::custom(
                "immutable input has an unclaimed trailing record",
            ));
        }
        Ok(())
    }
}

fn decode_error(source: serde_json::Error) -> CrawlError {
    CrawlError::Encode {
        table: "immutable result request replay".into(),
        source,
    }
}

fn validate_fragment(uri: &url::Url) -> CrawlResult<()> {
    let fragment = uri
        .fragment()
        .ok_or_else(|| invariant("request replay locator has no input binding"))?;
    if fragment.split(';').count() != 4 {
        return Err(invariant(
            "request replay locator has unclaimed input bindings",
        ));
    }
    fragment.split(';').try_for_each(|field| {
        let (key, value) = field
            .split_once('=')
            .ok_or_else(|| invariant("request replay input binding is malformed"))?;
        if value.is_empty()
            || !["requests", "pending", "input_sha256", "generation_sha256"].contains(&key)
        {
            return Err(invariant(
                "request replay locator has an unclaimed input binding",
            ));
        }
        Ok(())
    })
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
