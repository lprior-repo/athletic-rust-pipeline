use super::*;

#[test]
fn wrong_source_status_mime_request_method_time_or_raw_head_refuses() -> TestResult {
    let raw = research()?;
    for (requested, final_url, status) in [
        (
            "https://evil.invalid/hscoop.pdf",
            MHSAA_COOPS_SOURCE_URL,
            200,
        ),
        (
            MHSAA_COOPS_SOURCE_URL,
            "https://www.mhsaa.com/other.pdf",
            200,
        ),
        (MHSAA_COOPS_SOURCE_URL, MHSAA_COOPS_SOURCE_URL, 304),
    ] {
        check!(MhsaaCoopPdfCaptureV1::new(
            requested,
            final_url,
            status,
            raw.completed_at,
            PDF,
            provenance()?
        )
        .is_err());
    }
    for field in [
        "mime",
        "method",
        "time",
        "status",
        "length",
        "encoding",
        "duplicate_mime",
        "redirect",
        "control",
    ] {
        let mut changed = provenance()?;
        match field {
            "mime" => changed.content_type = "text/html; charset=utf-8".into(),
            "method" => changed.request_method = "POST".into(),
            "time" => changed.started_at = raw.completed_at + chrono::Duration::seconds(1),
            "status" => changed.response_header_bytes = b"HTTP/2 403\r\ncontent-type: application/pdf\r\n\r\n".to_vec(),
            "length" => changed.response_header_bytes = b"HTTP/2 200\r\ncontent-type: application/pdf\r\ncontent-length: 1\r\n\r\n".to_vec(),
            "encoding" => changed.response_header_bytes = b"HTTP/2 200\r\ncontent-type: application/pdf\r\ncontent-encoding: gzip\r\n\r\n".to_vec(),
            "duplicate_mime" => changed.response_header_bytes = b"HTTP/2 200\r\ncontent-type: application/pdf\r\ncontent-type: application/pdf\r\n\r\n".to_vec(),
            "redirect" => changed.response_header_bytes = b"HTTP/2 302\r\nlocation: /other\r\n\r\nHTTP/2 200\r\ncontent-type: application/pdf\r\n\r\n".to_vec(),
            "control" => changed.request_headers.push(("Accept".into(), "bad\nvalue".into())),
            _ => return Err("unexpected metadata mutation".into()),
        }
        check!(MhsaaCoopPdfCaptureV1::new(
            MHSAA_COOPS_SOURCE_URL,
            MHSAA_COOPS_SOURCE_URL,
            200,
            raw.completed_at,
            PDF,
            changed
        )
        .is_err());
    }
    Ok(())
}

#[test]
fn changed_hash_length_base64_version_interpretation_and_unknown_fields_refuse() -> TestResult {
    for path in [
        "hash",
        "length",
        "base64",
        "version",
        "interpretation",
        "unknown",
        "raw_unknown",
        "body_encoding",
    ] {
        let changed = altered(|value| match path {
            "hash" => value["capture"]["body_sha256"] = Value::from("0".repeat(64)),
            "length" => value["capture"]["body_bytes"] = Value::from(1),
            "base64" => value["capture"]["body_base64"] = Value::from("!!!!"),
            "version" => value["version"] = Value::from(2),
            "interpretation" => value["interpretation"] = Value::from("mhsaa_owned_pdf_v1"),
            "unknown" => value["accepted_region"] = Value::Bool(true),
            "raw_unknown" => value["capture"]["provenance"]["observed_http"] = Value::Bool(true),
            "body_encoding" => {
                value["capture"]["provenance"]["body_encoding"] = Value::from("gzip")
            }
            _ => {}
        })?;
        check!(MhsaaCoopPdfPacketV1::decode(&changed).is_err());
    }
    Ok(())
}

#[test]
fn packet_and_field_allocation_bounds_refuse_before_binary_or_semantic_decode() -> TestResult {
    let oversized = vec![
        0;
        MAX_MHSAA_COOP_PDF_PACKET_BYTES
            .checked_add(1)
            .ok_or("packet bound")?
    ];
    check!(MhsaaCoopPdfPacketV1::decode(&oversized).is_err());
    for field in ["encoded", "headers", "header_value", "dump"] {
        let changed = altered(|value| match field {
            "encoded" => {
                value["capture"]["body_base64"] = Value::from("A".repeat(MAX_BASE64_BYTES + 1))
            }
            "headers" => {
                value["capture"]["provenance"]["request_headers"] =
                    serde_json::json!(vec![("Accept", "*/*"); 33])
            }
            "header_value" => {
                value["capture"]["provenance"]["request_headers"] =
                    serde_json::json!([["Accept", "a".repeat(2049)]])
            }
            "dump" => {
                value["capture"]["provenance"]["response_header_bytes"] =
                    serde_json::json!(vec![32u8; 16 * 1024 + 1])
            }
            _ => {}
        })?;
        check!(changed.len() <= MAX_MHSAA_COOP_PDF_PACKET_BYTES);
        check!(MhsaaCoopPdfPacketV1::decode(&changed).is_err());
    }
    let raw = research()?;
    let oversized = vec![0; super::super::super::MAX_PDF_BYTES + 1];
    check!(MhsaaCoopPdfCaptureV1::new(
        MHSAA_COOPS_SOURCE_URL,
        MHSAA_COOPS_SOURCE_URL,
        200,
        raw.completed_at,
        &oversized,
        provenance()?
    )
    .is_err());
    Ok(())
}

#[test]
fn noncanonical_bytes_duplicate_fields_and_non_pdf_original_refuse() -> TestResult {
    let packet = packet()?;
    let encoded = packet.encode()?;
    let pretty = serde_json::to_vec_pretty(&packet)?;
    check!(MhsaaCoopPdfPacketV1::decode(&pretty).is_err());
    let text = String::from_utf8(encoded)?;
    let duplicated = text.replacen("\"version\":1", "\"version\":1,\"version\":1", 1);
    check!(MhsaaCoopPdfPacketV1::decode(duplicated.as_bytes()).is_err());
    let raw = research()?;
    let mut provenance = provenance()?;
    provenance.response_header_bytes =
        b"HTTP/2 200\r\ncontent-type: application/pdf\r\ncontent-length: 9\r\n\r\n".to_vec();
    let capture = MhsaaCoopPdfCaptureV1::new(
        MHSAA_COOPS_SOURCE_URL,
        MHSAA_COOPS_SOURCE_URL,
        200,
        raw.completed_at,
        b"not a PDF",
        provenance,
    )?;
    let packet = MhsaaCoopPdfPacketV1::new(capture)?;
    check!(packet.read().is_err());
    Ok(())
}
