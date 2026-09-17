use serde::{de::DeserializeOwned, Serialize};

/// Successful domain values must survive the same JSON boundary used by retained artifacts.
pub fn assert_json<T: Serialize + DeserializeOwned>(value: &T) {
    let roundtrip = serde_json::to_vec(value).and_then(|encoded| {
        let decoded: T = serde_json::from_slice(&encoded)?;
        serde_json::to_vec(&decoded).map(|reencoded| encoded == reencoded)
    });
    assert!(
        matches!(roundtrip, Ok(true)),
        "successful value changed across its persisted JSON boundary"
    );
}
