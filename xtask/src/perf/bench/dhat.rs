use anyhow::{bail, Context, Result};
use serde::de::{Error, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;
use std::io::{BufReader, Read};
use std::path::Path;

const MAX_PROFILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_PROFILE_POINTS: usize = 65_536;

#[derive(Deserialize)]
struct Profile {
    #[serde(rename = "dhatFileVersion")]
    version: u64,
    mode: String,
    pps: Totals,
}

#[derive(Deserialize)]
struct Point {
    tbk: u64,
    tb: u64,
}

struct Totals {
    blocks: u64,
    bytes: u64,
}

impl<'de> Deserialize<'de> for Totals {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        deserializer.deserialize_seq(PointsVisitor)
    }
}

struct PointsVisitor;

impl<'de> Visitor<'de> for PointsVisitor {
    type Value = Totals;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("at most 65536 DHAT heap profile points with u64 tbk and tb")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut points: A) -> std::result::Result<Totals, A::Error> {
        let mut totals = Totals {
            blocks: 0,
            bytes: 0,
        };
        for _ in 0..MAX_PROFILE_POINTS {
            let Some(point) = points.next_element::<Point>()? else {
                return Ok(totals);
            };
            totals.blocks = totals
                .blocks
                .checked_add(point.tbk)
                .ok_or_else(|| A::Error::custom("DHAT allocation count overflow"))?;
            totals.bytes = totals
                .bytes
                .checked_add(point.tb)
                .ok_or_else(|| A::Error::custom("DHAT allocated bytes overflow"))?;
        }
        if points.next_element::<serde::de::IgnoredAny>()?.is_some() {
            return Err(A::Error::custom("DHAT profile exceeds 65536 points"));
        }
        Ok(totals)
    }
}

pub(super) fn read(path: &Path) -> Result<(u64, u64)> {
    let file = std::fs::File::open(path).context("reading required DHAT profile")?;
    parse(file)
}

fn parse(input: impl Read) -> Result<(u64, u64)> {
    let mut limited = input.take(MAX_PROFILE_BYTES + 1);
    let profile: Profile = serde_json::from_reader(BufReader::new(&mut limited))
        .context("invalid DHAT heap profile JSON")?;
    if limited.limit() == 0 {
        bail!("DHAT profile exceeds 16 MiB");
    }
    if profile.version != 2 || profile.mode != "heap" {
        bail!("DHAT profile requires dhatFileVersion 2 and mode heap");
    }
    if profile.pps.blocks == 0 || profile.pps.bytes == 0 {
        bail!("DHAT allocation count and allocated bytes must be positive");
    }
    Ok((profile.pps.blocks, profile.pps.bytes))
}

#[cfg(test)]
#[path = "dhat_tests.rs"]
mod tests;
