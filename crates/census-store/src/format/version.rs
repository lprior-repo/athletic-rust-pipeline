use super::{StoreFormat, Verdict, KEY_FORMAT_VERSION, STORE_SCHEMA_VERSION};

pub(super) fn classify(format: &StoreFormat, version: u32, key_format: u32) -> Verdict {
    if format.created_by.is_none() || format.created_at.is_none() {
        return Verdict::Unknown("creation lineage rows are missing".to_string());
    }
    if version < STORE_SCHEMA_VERSION || key_format < KEY_FORMAT_VERSION {
        return Verdict::Migrate(format!(
            "the store records schema version {version} and key format {key_format}"
        ));
    }
    Verdict::Current
}
