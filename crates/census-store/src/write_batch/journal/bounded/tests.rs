use super::*;

#[derive(Serialize)]
struct OutOfOrder {
    z: u32,
    a: u32,
}

#[test]
fn bounded_encoder_preserves_the_existing_sorted_envelope_and_payload(
) -> Result<(), Box<dyn std::error::Error>> {
    let (key, bytes) = encode_publication_entry("phase", "key", &OutOfOrder { z: 2, a: 1 })?;
    if key != crate::Store::journal_key("phase", "key") || key != b"phase\0key" {
        return Err("publication journal changed logical-key framing".into());
    }
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    let at = value
        .get("at")
        .and_then(serde_json::Value::as_str)
        .ok_or("timestamp absent")?;
    let expected = format!(
        "{{\"at\":{},\"key\":\"key\",\"payload\":{{\"a\":1,\"z\":2}}}}",
        serde_json::to_string(at)?
    );
    if bytes != expected.as_bytes() {
        return Err("bounded encoding changed the historical canonical envelope ordering".into());
    }
    Ok(())
}

#[test]
fn geometric_capacity_growth_never_exceeds_the_declared_byte_limit(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = LimitedWriter::new(1024);
    for _ in 0..1024 {
        writer.write_all(b"x")?;
        if writer.bytes.capacity() > writer.limit {
            return Err("publication writer reserved above its limit".into());
        }
    }
    if writer.write_all(b"x").is_ok() || writer.bytes.len() != 1024 {
        return Err("publication writer accepted a byte past its limit".into());
    }
    Ok(())
}
