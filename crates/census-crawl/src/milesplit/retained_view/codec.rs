use super::*;
use serde::de::{Error, SeqAccess, Visitor};
use serde_json::value::RawValue;
use std::fmt;
use std::io::Write;
use std::ops::ControlFlow;

pub(super) fn entries<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<Entry>, D::Error> {
    struct Entries;
    impl<'de> Visitor<'de> for Entries {
        type Value = Vec<Entry>;
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("bounded retained journal entries")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
            let result = (0..=MAX_ENTRIES).try_fold(Vec::new(), |mut entries, index| {
                let raw = match sequence.next_element::<&'de RawValue>() {
                    Ok(Some(raw)) => raw,
                    Ok(None) => return ControlFlow::Break(Ok(entries)),
                    Err(error) => return ControlFlow::Break(Err(error)),
                };
                if index == MAX_ENTRIES || raw.get().len() > MAX_PAYLOAD + 2048 {
                    return ControlFlow::Break(Err(A::Error::custom(
                        "retained journal entry envelope exceeds its admission budget",
                    )));
                }
                let entry = match serde_json::from_str::<Entry>(raw.get()) {
                    Ok(entry) => entry,
                    Err(error) => return ControlFlow::Break(Err(A::Error::custom(error))),
                };
                if entries.len() == entries.capacity() {
                    let target = entries
                        .capacity()
                        .saturating_mul(2)
                        .max(entries.len().saturating_add(1))
                        .min(MAX_ENTRIES);
                    let Some(reserved) = target.checked_sub(entries.len()) else {
                        return ControlFlow::Break(Err(A::Error::custom(
                            "retained journal entry allocation",
                        )));
                    };
                    if entries.try_reserve_exact(reserved).is_err() {
                        return ControlFlow::Break(Err(A::Error::custom(
                            "retained journal entry allocation",
                        )));
                    }
                }
                entries.push(entry);
                ControlFlow::Continue(entries)
            });
            match result {
                ControlFlow::Break(result) => result,
                ControlFlow::Continue(_) => {
                    Err(A::Error::custom("retained journal entry count exceeded"))
                }
            }
        }
    }
    deserializer.deserialize_seq(Entries)
}

#[derive(Default)]
pub(super) struct LimitedBytes(Vec<u8>);
impl LimitedBytes {
    pub(super) fn finish(self) -> Vec<u8> {
        self.0
    }
}
impl Write for LimitedBytes {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let needed = self
            .0
            .len()
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("retained journal size overflow"))?;
        if needed > MAX_BYTES {
            return Err(std::io::Error::other(
                "retained journal encoding exceeds byte budget",
            ));
        }
        if needed > self.0.capacity() {
            let target = self
                .0
                .capacity()
                .saturating_mul(2)
                .max(needed)
                .min(MAX_BYTES);
            self.0
                .try_reserve_exact(
                    target.checked_sub(self.0.len()).ok_or_else(|| {
                        std::io::Error::other("retained journal capacity underflow")
                    })?,
                )
                .map_err(std::io::Error::other)?;
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(super) fn canonical_digest(bundle: &RetainedJournalBundle) -> StoreResult<String> {
    use sha2::{Digest, Sha256};
    struct Hash {
        hash: Sha256,
        count: usize,
    }
    impl Write for Hash {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let count = self
                .count
                .checked_add(bytes.len())
                .ok_or_else(|| std::io::Error::other("retained journal size overflow"))?;
            if count > MAX_BYTES {
                return Err(std::io::Error::other(
                    "retained journal encoding exceeds byte budget",
                ));
            }
            self.hash.update(bytes);
            self.count = count;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut output = Hash {
        hash: Sha256::new(),
        count: 0,
    };
    serde_json::to_writer(&mut output, bundle).map_err(encoding)?;
    Ok(format!("{:x}", output.hash.finalize()))
}
