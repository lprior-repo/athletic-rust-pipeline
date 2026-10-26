use super::super::{OwnedPerformance, OwnedRejection, MAX_OWNED_BODY_BYTES, MAX_OWNED_ROWS};
use super::error::{
    OwnedParseArithmetic, OwnedParseLocator, OwnedParseResource, OwnedParseResourceFailure,
};

pub(super) const MAX_DECODE_BYTES: usize = 512 * 1024 * 1024;
const MAX_WORK: usize = 2_000_000;
const MAX_DEPTH: usize = 127;
const VALUE_NODE_BYTES: usize = 512;
const TEXT_STORAGE_COPIES: usize = 32;

type AdmissionResult<T> = Result<T, OwnedParseResourceFailure>;

#[derive(Clone, Copy, Default)]
struct Frame {
    array: bool,
    next: bool,
    values: usize,
}

struct Admission {
    frames: [Frame; MAX_DEPTH],
    depth: usize,
    quoted: bool,
    escaped: bool,
    tokens: usize,
}

pub(super) fn check(body: &[u8]) -> AdmissionResult<()> {
    OwnedParseResourceFailure::limit(
        OwnedParseResource::CaptureBytes,
        OwnedParseLocator::Document,
        body.len(),
        MAX_OWNED_BODY_BYTES,
    )?;
    let state = Admission {
        frames: [Frame::default(); MAX_DEPTH],
        depth: 0,
        quoted: false,
        escaped: false,
        tokens: 0,
    };
    let admitted = body
        .iter()
        .enumerate()
        .try_fold(state, |mut state, (offset, byte)| {
            state.accept(*byte, OwnedParseLocator::Byte { offset })?;
            Ok::<_, OwnedParseResourceFailure>(state)
        })?;
    let requested = decoded_bytes(body.len(), admitted.tokens, MAX_OWNED_ROWS)?;
    OwnedParseResourceFailure::limit(
        OwnedParseResource::DecodedAllocation,
        OwnedParseLocator::Document,
        requested,
        MAX_DECODE_BYTES,
    )
}

fn decoded_bytes(body_bytes: usize, tokens: usize, rows: usize) -> AdmissionResult<usize> {
    let text = body_bytes.checked_mul(TEXT_STORAGE_COPIES).ok_or_else(|| {
        arithmetic(
            OwnedParseResource::DecodedAllocation,
            OwnedParseLocator::Document,
            OwnedParseArithmetic::TextBytes,
        )
    })?;
    let nodes = tokens.checked_mul(VALUE_NODE_BYTES).ok_or_else(|| {
        arithmetic(
            OwnedParseResource::DecodedAllocation,
            OwnedParseLocator::Document,
            OwnedParseArithmetic::NodeBytes,
        )
    })?;
    let row_bytes = std::mem::size_of::<OwnedPerformance>()
        .checked_add(std::mem::size_of::<OwnedRejection>())
        .ok_or_else(|| {
            arithmetic(
                OwnedParseResource::DecodedAllocation,
                OwnedParseLocator::Data,
                OwnedParseArithmetic::RowBytes,
            )
        })?;
    let vectors = rows.checked_mul(row_bytes).ok_or_else(|| {
        arithmetic(
            OwnedParseResource::DecodedAllocation,
            OwnedParseLocator::Data,
            OwnedParseArithmetic::VectorBytes,
        )
    })?;
    text.checked_add(nodes)
        .and_then(|size| size.checked_add(vectors))
        .ok_or_else(|| {
            arithmetic(
                OwnedParseResource::DecodedAllocation,
                OwnedParseLocator::Document,
                OwnedParseArithmetic::TotalBytes,
            )
        })
}

impl Admission {
    fn accept(&mut self, byte: u8, locator: OwnedParseLocator) -> AdmissionResult<()> {
        if matches!(byte, b'[' | b']' | b'{' | b'}' | b':' | b',' | b'"') {
            self.tokens = self.tokens.checked_add(1).ok_or_else(|| {
                arithmetic(
                    OwnedParseResource::DecodeWork,
                    locator,
                    OwnedParseArithmetic::TokenIncrement,
                )
            })?;
            OwnedParseResourceFailure::limit(
                OwnedParseResource::DecodeWork,
                locator,
                self.tokens,
                MAX_WORK,
            )?;
        }
        if self.quoted {
            self.string(byte);
            return Ok(());
        }
        self.value(byte, locator)?;
        match byte {
            b'"' => self.quoted = true,
            b'[' | b'{' => self.open(byte == b'[', locator)?,
            b']' | b'}' => self.depth = self.depth.saturating_sub(1),
            b',' => {
                if let Some(frame) = self.top() {
                    frame.next = true;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn string(&mut self, byte: u8) {
        if self.escaped {
            self.escaped = false;
        } else if byte == b'\\' {
            self.escaped = true;
        } else if byte == b'"' {
            self.quoted = false;
        }
    }

    fn value(&mut self, byte: u8, locator: OwnedParseLocator) -> AdmissionResult<()> {
        let Some(frame) = self.top() else {
            return Ok(());
        };
        if frame.array && frame.next && !byte.is_ascii_whitespace() && !matches!(byte, b']' | b',')
        {
            frame.values = frame.values.checked_add(1).ok_or_else(|| {
                arithmetic(
                    OwnedParseResource::ArrayValues,
                    locator,
                    OwnedParseArithmetic::ArrayIncrement,
                )
            })?;
            OwnedParseResourceFailure::limit(
                OwnedParseResource::ArrayValues,
                locator,
                frame.values,
                MAX_OWNED_ROWS,
            )?;
            frame.next = false;
        }
        Ok(())
    }

    fn open(&mut self, array: bool, locator: OwnedParseLocator) -> AdmissionResult<()> {
        let requested = self.depth.checked_add(1).ok_or_else(|| {
            arithmetic(
                OwnedParseResource::JsonDepth,
                locator,
                OwnedParseArithmetic::DepthIncrement,
            )
        })?;
        OwnedParseResourceFailure::limit(
            OwnedParseResource::JsonDepth,
            locator,
            requested,
            MAX_DEPTH,
        )?;
        let frame = self.frames.get_mut(self.depth).ok_or_else(|| {
            arithmetic(
                OwnedParseResource::JsonDepth,
                locator,
                OwnedParseArithmetic::DepthFrame,
            )
        })?;
        *frame = Frame {
            array,
            next: true,
            values: 0,
        };
        self.depth = requested;
        Ok(())
    }

    fn top(&mut self) -> Option<&mut Frame> {
        self.depth
            .checked_sub(1)
            .and_then(|index| self.frames.get_mut(index))
    }
}

fn arithmetic(
    resource: OwnedParseResource,
    locator: OwnedParseLocator,
    operation: OwnedParseArithmetic,
) -> OwnedParseResourceFailure {
    OwnedParseResourceFailure::arithmetic(resource, locator, operation)
}

#[path = "parse/recording.rs"]
mod recording;
pub(in crate::milesplit::owned) use recording::recording;

#[cfg(test)]
#[path = "parse/admission_tests.rs"]
mod tests;
