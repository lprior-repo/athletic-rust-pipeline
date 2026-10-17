use super::gzip::{
    create_private, hex_digest, other, MeasureWriter, Summary, NORMAL_BODY, WIRE_DECODED,
    WIRE_ENCODED,
};
use flate2::read::GzDecoder;
use flate2::Crc;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

const WIRE_HEADER: [u8; 10] = [0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff];
const EMPTY_BLOCK: [u8; 5] = [0x00, 0x00, 0x00, 0xff, 0xff];
const EMPTY_BLOCK_CHUNK: usize = 65_535;
const FINAL_BLOCK_BYTES: u64 = 5;
const CRC_BYTES: u64 = 4;
const ISIZE_BYTES: u64 = 4;
const WIRE_BODY_LEN: usize = 13;

fn empty_block_count(body_len: u64) -> io::Result<u64> {
    let header = u64::try_from(WIRE_HEADER.len()).map_err(|_| other("wire header length"))?;
    let trailer = CRC_BYTES
        .checked_add(ISIZE_BYTES)
        .ok_or_else(|| other("wire trailer length"))?;
    let fixed = header
        .checked_add(FINAL_BLOCK_BYTES)
        .and_then(|value| value.checked_add(body_len))
        .and_then(|value| value.checked_add(trailer))
        .ok_or_else(|| other("wire fixed length"))?;
    let block_bytes = WIRE_ENCODED
        .checked_sub(fixed)
        .ok_or_else(|| other("wire size below fixed bytes"))?;
    let block_size = u64::try_from(EMPTY_BLOCK.len()).map_err(|_| other("wire block length"))?;
    let blocks = block_bytes
        .checked_div(block_size)
        .ok_or_else(|| other("wire block division"))?;
    if block_bytes.checked_rem(block_size) != Some(0) {
        return Err(other("wire block remainder"));
    }
    Ok(blocks)
}

fn write_empty_blocks(writer: &mut impl Write, mut blocks: u64) -> io::Result<()> {
    let mut buffer = [0u8; EMPTY_BLOCK_CHUNK];
    for chunk in buffer.chunks_exact_mut(EMPTY_BLOCK.len()) {
        for (slot, byte) in chunk.iter_mut().zip(EMPTY_BLOCK.iter()) {
            *slot = *byte;
        }
    }
    let per_chunk = buffer
        .len()
        .checked_div(EMPTY_BLOCK.len())
        .ok_or_else(|| other("wire blocks per chunk"))?;
    if per_chunk == 0 {
        return Err(other("wire chunk has no blocks"));
    }
    let per_chunk = u64::try_from(per_chunk).map_err(|_| other("wire chunk blocks"))?;
    while blocks >= per_chunk {
        writer.write_all(&buffer)?;
        blocks = blocks
            .checked_sub(per_chunk)
            .ok_or_else(|| other("wire block count"))?;
    }
    let tail_blocks = usize::try_from(blocks).map_err(|_| other("wire tail blocks"))?;
    let tail_len = tail_blocks
        .checked_mul(EMPTY_BLOCK.len())
        .ok_or_else(|| other("wire tail length"))?;
    let tail = buffer
        .get(..tail_len)
        .ok_or_else(|| other("wire tail bounds"))?;
    writer.write_all(tail)
}

pub(crate) fn write_wire(path: &Path) -> io::Result<Summary> {
    let body = NORMAL_BODY.as_bytes();
    if body.len() != WIRE_BODY_LEN {
        return Err(other("wire body length"));
    }
    let body_len = u64::try_from(body.len()).map_err(|_| other("wire decoded length"))?;
    if body_len != WIRE_DECODED {
        return Err(other("wire decoded count"));
    }
    let len16 = u16::try_from(body.len()).map_err(|_| other("wire body length"))?;
    let nlen = !len16;
    let [len_lo, len_hi] = len16.to_le_bytes();
    let [nlen_lo, nlen_hi] = nlen.to_le_bytes();
    let final_block = [0x01, len_lo, len_hi, nlen_lo, nlen_hi];
    let blocks = empty_block_count(body_len)?;
    let file = create_private(path)?;
    let mut writer = MeasureWriter::new(file, WIRE_ENCODED);
    writer.write_all(&WIRE_HEADER)?;
    write_empty_blocks(&mut writer, blocks)?;
    writer.write_all(&final_block)?;
    writer.write_all(body)?;
    let mut crc = Crc::new();
    crc.update(body);
    writer.write_all(&crc.sum().to_le_bytes())?;
    let isize = u32::try_from(body.len()).map_err(|_| other("wire isize"))?;
    writer.write_all(&isize.to_le_bytes())?;
    let (mut file, encoded, sha256) = writer.finish()?;
    if encoded != WIRE_ENCODED {
        return Err(other("wire encoded count"));
    }
    file.flush()?;
    file.sync_all()?;
    verify_wire(path, body)?;
    let mut decoded_hasher = Sha256::new();
    decoded_hasher.update(body);
    let decoded_sha256 = hex_digest(decoded_hasher)?;
    Ok(Summary {
        encoded,
        decoded: WIRE_DECODED,
        sha256,
        decoded_sha256,
    })
}

fn verify_wire(path: &Path, body: &[u8]) -> io::Result<()> {
    let file = File::open(path)?;
    let mut decoder = GzDecoder::new(file);
    let mut buffer = [0u8; WIRE_BODY_LEN];
    decoder.read_exact(&mut buffer)?;
    if buffer.as_slice() != body {
        return Err(other("wire decoded body"));
    }
    let mut extra = [0u8; 1];
    if decoder.read(&mut extra)? != 0 {
        return Err(other("wire trailing bytes"));
    }
    Ok(())
}
