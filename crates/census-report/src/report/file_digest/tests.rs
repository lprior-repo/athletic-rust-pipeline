use super::{hash_reader, FileDigestError};
use std::io::{Cursor, Read};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn exact_budget_hashes_every_byte_and_checks_eof() -> TestResult {
    let mut input = Cursor::new(b"abc");
    let digest = hash_reader(&mut input, 3)?;
    check!(eq; digest.bytes, 3);
    check!(eq; digest.sha256, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    check!(eq; input.position(), 3);
    Ok(())
}

#[test]
fn zero_budget_accepts_only_an_empty_reader() -> TestResult {
    let empty = hash_reader(&mut Cursor::new(b""), 0)?;
    check!(eq; empty.bytes, 0);
    check!(eq; empty.sha256, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    let mut input = Cursor::new(b"abc");
    check!(matches!(
        hash_reader(&mut input, 0),
        Err(FileDigestError::BudgetExceeded)
    ));
    check!(eq; input.position(), 1);
    Ok(())
}

#[test]
fn oversized_reader_stops_after_one_byte_beyond_the_budget() -> TestResult {
    let mut input = Cursor::new(vec![42_u8; 131_072]);
    check!(matches!(
        hash_reader(&mut input, 65_536),
        Err(FileDigestError::BudgetExceeded)
    ));
    check!(eq; input.position(), 65_537);
    Ok(())
}

struct ShortReader {
    input: Cursor<&'static [u8]>,
    reads: usize,
}

impl Read for ShortReader {
    fn read(&mut self, target: &mut [u8]) -> std::io::Result<usize> {
        self.reads = self
            .reads
            .checked_add(1)
            .ok_or_else(|| std::io::Error::other("test read counter exhausted"))?;
        let target = target
            .get_mut(..1)
            .ok_or_else(|| std::io::Error::other("empty read buffer"))?;
        self.input.read(target)
    }
}

#[test]
fn short_reads_are_counted_until_exact_budget_eof() -> TestResult {
    let mut input = ShortReader {
        input: Cursor::new(b"abc"),
        reads: 0,
    };
    let digest = hash_reader(&mut input, 3)?;
    check!(eq; digest.bytes, 3);
    check!(eq; input.reads, 4);
    check!(eq; digest.sha256, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    Ok(())
}

struct InvalidReadLength;

impl Read for InvalidReadLength {
    fn read(&mut self, target: &mut [u8]) -> std::io::Result<usize> {
        target
            .len()
            .checked_add(1)
            .ok_or_else(|| std::io::Error::other("test read length overflow"))
    }
}

#[test]
fn an_invalid_reader_length_is_a_typed_refusal() -> TestResult {
    check!(matches!(
        hash_reader(&mut InvalidReadLength, 3),
        Err(FileDigestError::InvalidReadLength)
    ));
    Ok(())
}

struct BrokenReader;

impl Read for BrokenReader {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("fixture read failure"))
    }
}

#[test]
fn reader_failures_are_preserved() -> TestResult {
    check!(matches!(
        hash_reader(&mut BrokenReader, 3),
        Err(FileDigestError::Io(ref error)) if error.to_string() == "fixture read failure"
    ));
    Ok(())
}
