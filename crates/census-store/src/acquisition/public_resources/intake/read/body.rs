use super::super::{refused, MAX_BODY};
use crate::StoreResult;
use std::io::Read;

pub(super) fn read(path: &std::path::Path) -> StoreResult<Vec<u8>> {
    let file = std::fs::File::open(path).map_err(|_| refused("public school intake body open"))?;
    let maximum = u64::try_from(MAX_BODY + 1).map_err(|_| crate::StoreError::CounterOverflow)?;
    let mut body = Vec::new();
    file.take(maximum)
        .read_to_end(&mut body)
        .map_err(|_| refused("public school intake body read"))?;
    if body.is_empty() || body.len() > MAX_BODY {
        return Err(refused("public school intake body limit"));
    }
    Ok(body)
}
