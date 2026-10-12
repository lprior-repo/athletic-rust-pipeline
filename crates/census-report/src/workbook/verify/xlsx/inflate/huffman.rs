use super::Bits;
use crate::report::ReportResult;
use crate::workbook::verify::xlsx::Scope;

const MAX_BITS: usize = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Table {
    Alphabet,
    CodeLength,
    Fixed,
}

pub(super) struct Huffman {
    counts: [u16; MAX_BITS + 1],
    symbols: Vec<u16>,
}

impl Huffman {
    pub(super) fn construct(lengths: &[u8], table: Table, scope: &Scope<'_>) -> ReportResult<Self> {
        let mut counts = [0_u16; MAX_BITS + 1];
        for &length in lengths {
            let slot = usize::from(length);
            if slot > MAX_BITS {
                return Err(scope.corrupt("a code length exceeds fifteen bits"));
            }
            match counts.get_mut(slot) {
                Some(count) => *count = count.saturating_add(1),
                None => return Err(scope.corrupt("a code length is outside its table")),
            }
        }
        let mut left = 1_i32;
        let mut longest = 0_usize;
        for length in 1..=MAX_BITS {
            let count = i32::from(counts.get(length).copied().map_or(0, |count| count));
            left = left.saturating_mul(2).saturating_sub(count);
            if count != 0 {
                longest = length;
            }
        }
        if left < 0 {
            return Err(scope.corrupt("a deflate code table is over-subscribed"));
        }
        if left > 0 && longest > 1 && table == Table::Alphabet {
            return Err(scope.corrupt("a deflate code table is incomplete"));
        }
        let mut offsets = [0_u16; MAX_BITS + 2];
        let mut total = 0_u16;
        for length in 1..=MAX_BITS {
            if let Some(slot) = offsets.get_mut(length) {
                *slot = total;
            }
            total = total.saturating_add(counts.get(length).copied().map_or(0, |count| count));
        }
        let mut symbols = vec![0_u16; usize::from(total)];
        for (symbol, &length) in lengths.iter().enumerate() {
            if length == 0 {
                continue;
            }
            let slot = offsets
                .get_mut(usize::from(length))
                .ok_or_else(|| scope.corrupt("a code length is outside its table"))?;
            let target = usize::from(*slot);
            *slot = slot.saturating_add(1);
            let entry = u16::try_from(symbol)
                .map_err(|_| scope.corrupt("a code symbol does not fit its alphabet"))?;
            match symbols.get_mut(target) {
                Some(value) => *value = entry,
                None => return Err(scope.corrupt("a code table overflows its symbol count")),
            }
        }
        Ok(Self { counts, symbols })
    }

    pub(super) fn decode(&self, bits: &mut Bits<'_>, scope: &Scope<'_>) -> ReportResult<u16> {
        let mut code = 0_u32;
        let mut first = 0_u32;
        let mut index = 0_u32;
        for length in 1..=MAX_BITS {
            code |= bits.read(1, scope)?;
            let count = u32::from(self.counts.get(length).copied().map_or(0, |count| count));
            let matched = code.saturating_sub(first);
            if code >= first && matched < count {
                let offset = index.saturating_add(matched);
                let offset = usize::try_from(offset)
                    .map_err(|_| scope.corrupt("a code offset does not fit the readback"))?;
                return self
                    .symbols
                    .get(offset)
                    .copied()
                    .ok_or_else(|| scope.corrupt("a code table points outside its symbols"));
            }
            index = index.saturating_add(count);
            first = first.saturating_add(count).saturating_mul(2);
            code = code.saturating_mul(2);
        }
        Err(scope.corrupt("a deflate code is longer than fifteen bits"))
    }
}
