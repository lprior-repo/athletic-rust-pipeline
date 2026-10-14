use super::{decode_chunks, length, CorruptionClass, Decode, Store, StoreError, StoreResult};
use crate::journal::parse_chunk_key;
use crate::keys;
use crate::Table;
pub(super) fn journal_pass(decode: &mut Decode) -> StoreResult<()> {
    decode.set_pass("journal decode");
    let store = decode.store;
    for guard in store.journal.iter() {
        let (key, raw) = guard
            .into_inner()
            .map_err(|source| StoreError::Read { source })?;
        if !decode.charge(1, length(raw.len())) {
            return Ok(());
        }
        let bytes: &[u8] = key.as_ref();
        let Some(separator) = bytes.iter().position(|byte| *byte == 0) else {
            if !decode.note(
                CorruptionClass::Undecodable,
                format!("journal/{}", keys::key_label(bytes)),
                "the journal key carries no phase separator".to_string(),
            ) {
                return Ok(());
            }
            continue;
        };
        let phase = String::from_utf8_lossy(
            bytes
                .get(..separator)
                .map_or(&[][..], core::convert::identity),
        );
        let Some(locator) = bytes.get(separator.saturating_add(1)..) else {
            continue;
        };
        if let Some((logical, index)) = parse_chunk_key(locator) {
            let logical_key = Store::journal_key(&phase, &String::from_utf8_lossy(logical));
            if !store
                .journal
                .contains_key(logical_key)
                .map_err(|source| StoreError::Read { source })?
            {
                let mut subject = format!("journal/chunk/{}", keys::key_label(bytes));
                decode_chunks::read_single_chunk(decode, raw.as_ref(), index, &mut subject);
            }
            continue;
        }
        let locator = String::from_utf8_lossy(locator);
        super::decode_entry::entry(decode, &phase, &locator, &key, raw.as_ref())?;
    }
    Ok(())
}

pub(super) fn payload_pass(decode: &mut Decode) -> StoreResult<()> {
    decode.set_pass("payload decode");
    let store = decode.store;
    for table in Table::ALL {
        let prefix = store.layout().prefix(table);
        for guard in store.entities.prefix(prefix.as_slice()) {
            let (key, value) = guard
                .into_inner()
                .map_err(|source| StoreError::Read { source })?;
            if !decode.charge(1, length(value.len())) {
                return Ok(());
            }
            if let Err(error) = keys::observation_id(value.as_ref()) {
                if !decode.note(
                    CorruptionClass::Undecodable,
                    format!("row/{}/{}", table.file(), keys::key_label(&key)),
                    error.to_string(),
                ) {
                    return Ok(());
                }
            }
        }
    }
    Ok(())
}
