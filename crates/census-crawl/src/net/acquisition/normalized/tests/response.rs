use super::*;

#[test]
fn response_observes_api_status_url_protocol_headers_and_head_time_exactly() -> TestResult {
    let native = built_response(duplicates()?, reqwest::Version::HTTP_2, 429)?;
    let expected = native
        .headers()
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_bytes()))
        .collect::<Vec<_>>();
    let observed = super::super::response(&native, HEAD)?;
    let retained = observed
        .headers()
        .iter()
        .map(|header| (header.name(), header.value()))
        .collect::<Vec<_>>();
    check!(eq; retained, expected);
    check!(eq; observed.status(), 429);
    check!(eq; observed.protocol(), ObservedHttpProtocolV1::Http2);
    check!(eq; observed.response_url(), native.url().as_str());
    check!(eq; observed.received_at(), HEAD);
    let encoded = serde_json::to_vec(&observed)?;
    check!(eq; serde_json::from_slice::<ObservedHttpResponseV1>(&encoded)?, observed);
    check!(eq; native.headers().get_all("x-evidence").iter().count(), 3);
    Ok(())
}

#[test]
fn actual_response_version_maps_only_the_four_closed_protocols() -> TestResult {
    for (version, expected) in [
        (reqwest::Version::HTTP_10, ObservedHttpProtocolV1::Http10),
        (reqwest::Version::HTTP_11, ObservedHttpProtocolV1::Http11),
        (reqwest::Version::HTTP_2, ObservedHttpProtocolV1::Http2),
        (reqwest::Version::HTTP_3, ObservedHttpProtocolV1::Http3),
    ] {
        let native = built_response(HeaderMap::new(), version, 200)?;
        check!(eq; super::super::response(&native, HEAD)?.protocol(), expected);
    }
    let unsupported = built_response(HeaderMap::new(), reqwest::Version::HTTP_09, 200)?;
    check!(matches!(
        super::super::response(&unsupported, HEAD),
        Err(FetchError::CaptureDomain(CaptureDomainError::Invalid {
            field: "observed response protocol"
        }))
    ));
    Ok(())
}

#[test]
fn response_retains_identity_encoding_bytes_and_refuses_any_nonidentity_occurrence() -> TestResult {
    let mut headers = duplicates()?;
    headers.append(
        HeaderName::from_bytes(b"Content-Encoding")?,
        HeaderValue::from_bytes(b" \tIDENTITY\t ")?,
    );
    let native = built_response(headers, reqwest::Version::HTTP_11, 200)?;
    let observed = super::super::response(&native, HEAD)?;
    let encoding = observed
        .headers()
        .iter()
        .find(|header| header.name() == "content-encoding")
        .ok_or("retained encoding")?;
    check!(eq; encoding.value(), b" \tIDENTITY\t ");
    for invalid in [
        b"gzip".as_slice(),
        b"br",
        b"deflate",
        b"zstd",
        b"identity,gzip",
        b"",
        &[0xff],
    ] {
        let mut headers = HeaderMap::new();
        headers.append(
            HeaderName::from_bytes(b"content-encoding")?,
            HeaderValue::from_bytes(b"identity")?,
        );
        headers.append(
            HeaderName::from_bytes(b"content-encoding")?,
            HeaderValue::from_bytes(invalid)?,
        );
        let native = built_response(headers, reqwest::Version::HTTP_11, 200)?;
        check!(matches!(
            super::super::response(&native, HEAD),
            Err(FetchError::CaptureDomain(CaptureDomainError::Invalid {
                field: "observed content encoding"
            }))
        ));
        check!(eq; native.headers().get_all("content-encoding").iter().count(), 2);
    }
    Ok(())
}

#[test]
fn response_does_not_advance_or_relabel_invalid_observed_head_time() -> TestResult {
    let native = built_response(HeaderMap::new(), reqwest::Version::HTTP_11, 200)?;
    for invalid in [
        "",
        "2026-10-21",
        "2026-10-21T08:00:00+01:00",
        "1899-01-01T00:00:00Z",
    ] {
        check!(matches!(
            super::super::response(&native, invalid),
            Err(FetchError::CaptureDomain(_))
        ));
    }
    check!(eq; super::super::response(&native, HEAD)?.received_at(), HEAD);
    Ok(())
}
