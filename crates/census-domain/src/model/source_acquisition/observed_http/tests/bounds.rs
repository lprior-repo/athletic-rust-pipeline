use super::*;

#[test]
fn header_names_and_raw_values_have_exact_bounds_and_token_rules() -> TestResult {
    let name = "x".repeat(128);
    check!(ObservedHttpHeaderV1::new(&name, vec![0xff; 2048]).is_ok());
    check!(ObservedHttpHeaderV1::new(&"x".repeat(129), Vec::new()).is_err());
    check!(ObservedHttpHeaderV1::new("x", vec![b'a'; 2049]).is_err());
    check!(ObservedHttpHeaderV1::new("!#$%&'*+-.^_`|~012abc", Vec::new()).is_ok());
    for name in ["", "X-Source", "x:y", "x y", "x/", "é", "x\t"] {
        check!(ObservedHttpHeaderV1::new(name, Vec::new()).is_err());
    }
    for byte in (0..32).filter(|byte| *byte != b'\t').chain([0x7f]) {
        check!(ObservedHttpHeaderV1::new("x", vec![b'a', byte, 0xff]).is_err());
    }
    for byte in [b'\t', b' ', b'~', 0x80, 0xff] {
        let header = ObservedHttpHeaderV1::new("x", vec![byte])?;
        check!(eq; header.value(), &[byte]);
    }
    Ok(())
}

#[test]
fn each_side_admits_32_occurrences_and_refuses_the_next_on_construction_and_decode() -> TestResult {
    let headers = fixture::headers(32, 0, b'x')?;
    let hop = fixture::hop(0, headers.clone(), headers)?;
    let original = fixture::observed(vec![hop])?;
    for side in ["request", "response"] {
        let mut value = serde_json::to_value(&original)?;
        value["hops"][0][side]["headers"] = serde_json::to_value(fixture::headers(33, 0, b'x')?)?;
        check!(serde_json::from_value::<NormalizedObservedHttpV1>(value).is_err());
    }
    check!(
        ObservedHttpRequestV1::new(fixture::URL, fixture::DONE, fixture::headers(33, 0, b'x')?)
            .is_err()
    );
    check!(fixture::response(fixture::headers(33, 0, b'x')?).is_err());
    Ok(())
}

#[test]
fn all_chain_header_names_and_values_share_one_exact_16_kib_ceiling() -> TestResult {
    let headers = fixture::headers(8, 2047, b'1')?;
    let hop = fixture::hop(0, headers, Vec::new())?;
    let original = fixture::observed(vec![hop])?;
    check!(eq; original.hops()[0].request().headers().len(), 8);
    let mut one_over = serde_json::to_value(&original)?;
    one_over["hops"][0]["response"]["headers"] =
        serde_json::to_value(fixture::headers(1, 0, b'1')?)?;
    check!(serde_json::from_value::<NormalizedObservedHttpV1>(one_over).is_err());
    let first = fixture::redirect(
        fixture::hop(0, fixture::headers(4, 2047, b'1')?, Vec::new())?,
        fixture::URL,
    )?;
    let location_bytes = "location"
        .len()
        .checked_add(fixture::URL.len())
        .ok_or("location bytes")?;
    let remaining = 2047usize
        .checked_sub(location_bytes)
        .ok_or("remaining header bytes")?;
    let mut headers = fixture::headers(3, 2047, b'1')?;
    headers.push(ObservedHttpHeaderV1::new("x", vec![b'1'; remaining])?);
    let second = fixture::hop(1, Vec::new(), headers.clone())?;
    check!(fixture::observed(vec![first.clone(), second]).is_ok());
    headers.push(ObservedHttpHeaderV1::new("y", Vec::new())?);
    let second = fixture::hop(1, Vec::new(), headers)?;
    check!(fixture::observed(vec![first, second]).is_err());
    Ok(())
}

#[test]
fn body_length_and_url_exact_limits_refuse_one_over_and_unsupported_urls() -> TestResult {
    let hops = vec![fixture::hop(0, Vec::new(), Vec::new())?];
    check!(NormalizedObservedHttpV1::new(
        hops.clone(),
        fixture::DONE,
        fixture::digest()?,
        32 * 1024 * 1024
    )
    .is_ok());
    check!(NormalizedObservedHttpV1::new(
        hops,
        fixture::DONE,
        fixture::digest()?,
        32 * 1024 * 1024 + 1
    )
    .is_err());
    let prefix = "https://example.test/";
    let pad = 2048usize
        .checked_sub(prefix.len())
        .ok_or("fixture URL length")?;
    let exact = format!("{prefix}{}", "x".repeat(pad));
    check!(eq; exact.len(), 2048);
    check!(ObservedHttpRequestV1::new(&exact, fixture::DONE, Vec::new()).is_ok());
    check!(ObservedHttpRequestV1::new(&format!("{exact}x"), fixture::DONE, Vec::new()).is_err());
    for url in [
        "ftp://example.test/source",
        "https://user@example.test/source",
        "https://example.test/source#fragment",
        "not a URL",
    ] {
        check!(ObservedHttpRequestV1::new(url, fixture::DONE, Vec::new()).is_err());
    }
    for status in [0, 99, 600, u16::MAX] {
        check!(ObservedHttpResponseV1::new(
            ObservedHttpProtocolV1::Http11,
            status,
            fixture::URL,
            fixture::DONE,
            Vec::new()
        )
        .is_err());
    }
    Ok(())
}

#[test]
fn full_encoded_metadata_admits_exact_64_kib_and_refuses_one_extra_encoded_byte() -> TestResult {
    let mut value = serde_json::to_value(fixture::original()?)?;
    *fixture::request_headers(&mut value)? = serde_json::to_value(fixture::headers(8, 2047, 255)?)?;
    let excess = serde_json::to_vec(&value)?
        .len()
        .checked_sub(65_536)
        .ok_or("fixture must exceed encoded cap")?;
    let values = value
        .pointer_mut("/hops/0/request/headers/0/value")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("fixture byte values")?;
    check!(excess > 0 && excess <= values.len());
    for byte in values.iter_mut().take(excess) {
        *byte = serde_json::json!(99);
    }
    check!(eq; serde_json::to_vec(&value)?.len(), 65_536);
    let exact: NormalizedObservedHttpV1 = serde_json::from_value(value.clone())?;
    check!(eq; serde_json::to_vec(&exact)?.len(), 65_536);
    value["hops"][0]["request"]["headers"][0]["value"][0] = serde_json::json!(255);
    check!(eq; serde_json::to_vec(&value)?.len(), 65_537);
    check!(serde_json::from_value::<NormalizedObservedHttpV1>(value).is_err());
    Ok(())
}
