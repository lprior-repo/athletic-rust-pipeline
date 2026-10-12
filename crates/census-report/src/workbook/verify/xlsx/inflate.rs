use super::{Scope, Sink};
use crate::report::ReportResult;

mod huffman;
mod window;

use huffman::{Huffman, Table};
use window::Output;

const CODE_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];
const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LENGTH_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DISTANCE_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1_025, 1_537,
    2_049, 3_073, 4_097, 6_145, 8_193, 12_289, 16_385, 24_577,
];
const DISTANCE_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

pub(super) fn decode(
    data: &[u8],
    limit: u64,
    scope: &Scope<'_>,
    sink: &mut impl Sink,
) -> ReportResult<u64> {
    let mut bits = Bits::new(data);
    let mut out = Output::new(sink, limit, *scope);
    let (fixed_literal, fixed_distance) = fixed(scope)?;
    loop {
        let last = bits.read(1, scope)? == 1;
        match bits.read(2, scope)? {
            0 => stored(&mut bits, &mut out, scope)?,
            1 => symbols(&mut bits, &fixed_literal, &fixed_distance, &mut out, scope)?,
            2 => {
                let (literal, distance) = dynamic(&mut bits, scope)?;
                symbols(&mut bits, &literal, &distance, &mut out, scope)?;
            }
            _ => {
                return Err(scope.corrupt("the deflate stream declares a reserved block type"));
            }
        }
        if last {
            break;
        }
    }
    out.finish()?;
    Ok(out.total)
}

fn fixed(scope: &Scope<'_>) -> ReportResult<(Huffman, Huffman)> {
    let mut literal = [0_u8; 288];
    for (symbol, length) in literal.iter_mut().enumerate() {
        *length = match symbol {
            0..=143 => 8,
            144..=255 => 9,
            256..=279 => 7,
            _ => 8,
        };
    }
    let distance = [5_u8; 30];
    let literal = Huffman::construct(&literal, Table::Fixed, scope)?;
    Ok((literal, Huffman::construct(&distance, Table::Fixed, scope)?))
}

fn dynamic(bits: &mut Bits<'_>, scope: &Scope<'_>) -> ReportResult<(Huffman, Huffman)> {
    let literals = usize::try_from(bits.read(5, scope)?)
        .map_or(0, core::convert::identity)
        .saturating_add(257);
    let distances = usize::try_from(bits.read(5, scope)?)
        .map_or(0, core::convert::identity)
        .saturating_add(1);
    let codes = usize::try_from(bits.read(4, scope)?)
        .map_or(0, core::convert::identity)
        .saturating_add(4);
    let lengths = read_code_lengths(bits, codes, scope)?;
    let table = Huffman::construct(&lengths, Table::CodeLength, scope)?;
    let total = literals.saturating_add(distances);
    let flattened = decode_flattened_code_lengths(bits, &table, total, scope)?;
    let literal = flattened
        .get(..literals)
        .ok_or_else(|| scope.corrupt("the literal code table is shorter than declared"))?;
    let distance = flattened
        .get(literals..)
        .ok_or_else(|| scope.corrupt("the distance code table is shorter than declared"))?;
    let literal = Huffman::construct(literal, Table::Alphabet, scope)?;
    Ok((
        literal,
        Huffman::construct(distance, Table::Alphabet, scope)?,
    ))
}

fn read_code_lengths(
    bits: &mut Bits<'_>,
    codes: usize,
    scope: &Scope<'_>,
) -> ReportResult<[u8; 19]> {
    let mut lengths = [0_u8; 19];
    for index in 0..codes {
        let position = *CODE_ORDER
            .get(index)
            .ok_or_else(|| scope.corrupt("the code length order runs past its table"))?;
        let value = u8::try_from(bits.read(3, scope)?)
            .map_err(|_| scope.corrupt("a code length does not fit a byte"))?;
        match lengths.get_mut(position) {
            Some(slot) => *slot = value,
            None => return Err(scope.corrupt("a code length index is outside its table")),
        }
    }
    Ok(lengths)
}

fn decode_flattened_code_lengths(
    bits: &mut Bits<'_>,
    table: &Huffman,
    total: usize,
    scope: &Scope<'_>,
) -> ReportResult<Vec<u8>> {
    let mut flattened = vec![0_u8; total];
    let mut index = 0_usize;
    while index < total {
        let symbol = table.decode(bits, scope)?;
        match symbol {
            0..=15 => {
                let value = u8::try_from(symbol)
                    .map_err(|_| scope.corrupt("a code length does not fit a byte"))?;
                match flattened.get_mut(index) {
                    Some(slot) => *slot = value,
                    None => return Err(scope.corrupt("a code length runs past its table")),
                }
                index = index.saturating_add(1);
            }
            16..=18 => {
                let span = repeat_span(bits, symbol, &flattened, index, scope)?;
                let next = index.saturating_add(span.0);
                let target = flattened
                    .get_mut(index..next)
                    .ok_or_else(|| scope.corrupt("a code length repeat runs past its table"))?;
                for slot in target {
                    *slot = span.1;
                }
                index = next;
            }
            _ => return Err(scope.corrupt("a code length symbol is outside its alphabet")),
        }
    }
    Ok(flattened)
}

fn repeat_span(
    bits: &mut Bits<'_>,
    symbol: u16,
    flattened: &[u8],
    index: usize,
    scope: &Scope<'_>,
) -> ReportResult<(usize, u8)> {
    match symbol {
        16 => {
            let previous = index
                .checked_sub(1)
                .and_then(|previous| flattened.get(previous).copied())
                .ok_or_else(|| scope.corrupt("a repeat code has no previous code length"))?;
            let repeat = usize::try_from(bits.read(2, scope)?).map_or(0, core::convert::identity);
            Ok((repeat.saturating_add(3), previous))
        }
        17 => {
            let repeat = usize::try_from(bits.read(3, scope)?).map_or(0, core::convert::identity);
            Ok((repeat.saturating_add(3), 0))
        }
        _ => {
            let repeat = usize::try_from(bits.read(7, scope)?).map_or(0, core::convert::identity);
            Ok((repeat.saturating_add(11), 0))
        }
    }
}

fn stored<S: Sink>(
    bits: &mut Bits<'_>,
    out: &mut Output<'_, S>,
    scope: &Scope<'_>,
) -> ReportResult<()> {
    bits.align();
    let length = bits.read(16, scope)?;
    let complement = bits.read(16, scope)?;
    if length ^ complement != 0xFFFF {
        return Err(scope.corrupt("a stored block length does not complement its pair"));
    }
    for _ in 0..length {
        let value = u8::try_from(bits.read(8, scope)?)
            .map_err(|_| scope.corrupt("a stored block byte does not fit a byte"))?;
        out.push(value)?;
    }
    Ok(())
}

fn symbols<S: Sink>(
    bits: &mut Bits<'_>,
    literal: &Huffman,
    distance: &Huffman,
    out: &mut Output<'_, S>,
    scope: &Scope<'_>,
) -> ReportResult<()> {
    loop {
        let symbol = literal.decode(bits, scope)?;
        match symbol {
            0..=255 => {
                let value = u8::try_from(symbol)
                    .map_err(|_| scope.corrupt("a literal symbol does not fit a byte"))?;
                out.push(value)?;
            }
            256 => return Ok(()),
            257..=285 => {
                let index = symbol
                    .checked_sub(257)
                    .ok_or_else(|| scope.corrupt("a length symbol is outside its alphabet"))?;
                let length = length_of(bits, index, scope)?;
                let code = distance.decode(bits, scope)?;
                let span = span_of(code, bits, scope)?;
                out.copy(span, length)?;
            }
            _ => return Err(scope.corrupt("a literal symbol is outside its alphabet")),
        }
    }
}

fn length_of(bits: &mut Bits<'_>, index: u16, scope: &Scope<'_>) -> ReportResult<usize> {
    let slot = *LENGTH_EXTRA
        .get(usize::from(index))
        .ok_or_else(|| scope.corrupt("a length symbol is outside its alphabet"))?;
    let base = *LENGTH_BASE
        .get(usize::from(index))
        .ok_or_else(|| scope.corrupt("a length symbol is outside its alphabet"))?;
    let extra = bits.read(u32::from(slot), scope)?;
    Ok(usize::from(base).saturating_add(usize::try_from(extra).map_or(0, core::convert::identity)))
}

fn span_of(code: u16, bits: &mut Bits<'_>, scope: &Scope<'_>) -> ReportResult<usize> {
    let slot = *DISTANCE_EXTRA
        .get(usize::from(code))
        .ok_or_else(|| scope.corrupt("a distance symbol is outside its alphabet"))?;
    let base = *DISTANCE_BASE
        .get(usize::from(code))
        .ok_or_else(|| scope.corrupt("a distance symbol is outside its alphabet"))?;
    let extra = bits.read(u32::from(slot), scope)?;
    Ok(usize::from(base).saturating_add(usize::try_from(extra).map_or(0, core::convert::identity)))
}

struct Bits<'a> {
    data: &'a [u8],
    position: usize,
    bit: u32,
}

impl<'a> Bits<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            position: 0,
            bit: 0,
        }
    }

    fn read(&mut self, count: u32, scope: &Scope<'_>) -> ReportResult<u32> {
        let mut value = 0_u32;
        for index in 0..count {
            let byte = self
                .data
                .get(self.position)
                .copied()
                .ok_or_else(|| scope.corrupt("the deflate stream ends inside a symbol"))?;
            let taken = (u32::from(byte) >> self.bit) & 1;
            value |= taken << index;
            self.bit = self.bit.saturating_add(1);
            if self.bit == 8 {
                self.bit = 0;
                self.position = self.position.saturating_add(1);
            }
        }
        Ok(value)
    }

    fn align(&mut self) {
        if self.bit != 0 {
            self.bit = 0;
            self.position = self.position.saturating_add(1);
        }
    }
}
