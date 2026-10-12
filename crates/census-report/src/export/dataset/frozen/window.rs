use super::super::codec::{checked_add, count, FrozenCodecError, FrozenCodecResult};
use super::ChunkReader;
use census_store::Entity;

const WINDOW_ROWS: u64 = 25_000;
const WINDOW_ENCODED_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Copy)]
pub struct EncodedWindowLimit {
    rows: u64,
    encoded_bytes: u64,
}

impl EncodedWindowLimit {
    pub fn new(rows: u64, encoded_bytes: u64) -> FrozenCodecResult<Self> {
        check_limit("frozen window records", rows, WINDOW_ROWS)?;
        check_limit(
            "frozen window encoded bytes",
            encoded_bytes,
            WINDOW_ENCODED_BYTES,
        )?;
        Ok(Self {
            rows,
            encoded_bytes,
        })
    }

    pub fn test_profile() -> FrozenCodecResult<Self> {
        Self::new(WINDOW_ROWS, WINDOW_ENCODED_BYTES)
    }
}

fn check_limit(what: &'static str, needed: u64, available: u64) -> FrozenCodecResult<()> {
    if needed == 0 || needed > available {
        return Err(FrozenCodecError::BudgetExceeded {
            what,
            needed,
            available,
        });
    }
    Ok(())
}

#[derive(Default, Debug, PartialEq, Eq)]
pub struct EncodedWindowCounts {
    pub rows: u64,
    pub encoded_bytes: u64,
}

impl ChunkReader<'_> {
    pub fn visit_window<T: Entity>(
        &mut self,
        limit: EncodedWindowLimit,
        mut visit: impl FnMut(&T) -> FrozenCodecResult<()>,
    ) -> FrozenCodecResult<EncodedWindowCounts> {
        let mut counts = EncodedWindowCounts::default();
        while counts.rows < limit.rows {
            let Some(line) = self.peek_line()? else {
                break;
            };
            let bytes = checked_add("frozen window line", count(line.len())?, 1)?;
            let needed = checked_add("frozen window encoded bytes", counts.encoded_bytes, bytes)?;
            if needed > limit.encoded_bytes {
                if counts.rows == 0 {
                    return Err(FrozenCodecError::BudgetExceeded {
                        what: "frozen window encoded bytes",
                        needed,
                        available: limit.encoded_bytes,
                    });
                }
                break;
            }
            let Some(decoded) = self.next_record::<T>()? else {
                break;
            };
            visit(&decoded.payload)?;
            counts.rows = checked_add("frozen window records", counts.rows, 1)?;
            counts.encoded_bytes = needed;
        }
        Ok(counts)
    }
}
