use std::io::{ErrorKind, Read};
use std::path::Path;

pub(super) fn with_bytes<const N: usize, T>(
    path: &Path,
    parse: impl FnOnce(&[u8]) -> T,
) -> std::io::Result<T> {
    let max = N
        .checked_sub(1)
        .ok_or_else(|| std::io::Error::new(ErrorKind::InvalidInput, "empty kernel read buffer"))?;
    let mut file = std::fs::File::open(path)?;
    let mut buffer = [0_u8; N];
    let mut used = 0_usize;
    for _ in 0..N {
        let remaining = buffer.get_mut(used..).ok_or_else(oversized)?;
        let count = match file.read(remaining) {
            Ok(count) => count,
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if count == 0 {
            return buffer.get(..used).map(parse).ok_or_else(oversized);
        }
        used = used.checked_add(count).ok_or_else(oversized)?;
        if used > max {
            return Err(oversized());
        }
    }
    Err(std::io::Error::new(
        ErrorKind::Interrupted,
        "kernel read operation budget exhausted",
    ))
}

fn oversized() -> std::io::Error {
    std::io::Error::new(
        ErrorKind::FileTooLarge,
        "kernel file exceeds declared byte limit",
    )
}
