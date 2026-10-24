use super::*;

#[test]
fn retained_redirect_chain_binds_actual_capture_and_preserves_observed_bytes() -> TestResult {
    let (capture, expected) = fixture::original()?;
    let bytes = fixture::encoded(&expected)?;
    let held = observed(&bytes, &capture)?.ok_or("normalized metadata")?;
    check!(eq; held, expected);
    check!(eq; held.hops().len(), 2);
    check!(eq; held.completed_at(), DONE);
    check!(eq; capture.fetched_at, HEAD);
    check!(ne; capture.fetched_at, held.completed_at());
    let last = held.hops().last().ok_or("final observed hop")?;
    let values = last
        .response()
        .headers()
        .iter()
        .filter(|header| header.name() == "x-source")
        .map(|header| header.value())
        .collect::<Vec<_>>();
    check!(eq; values, vec![b"first".as_slice(), &[0x80, 0xff]]);
    check!(eq; held.body_bytes(), u64::try_from(capture.body.len())?);
    check!(eq; held.body_digest().as_str(), crate::net::cache::content_digest(&capture.body));
    check!(eq; fixture::encoded(&held)?, bytes);
    let mut cached = capture.clone();
    cached.from_cache = true;
    check!(eq; observed(&bytes, &cached)?, Some(expected));
    Ok(())
}

type BodyMutation<'a> = (&'a str, fn(&mut FetchOutcome));

#[test]
fn changed_capture_request_response_or_declared_body_binding_refuses() -> TestResult {
    let (capture, expected) = fixture::original()?;
    let bytes = fixture::encoded(&expected)?;
    let mutations: [BodyMutation<'_>; 9] = [
        ("method", |value| value.method = "POST".into()),
        ("initial URL", |value| value.url = FINAL.into()),
        ("missing final URL", |value| value.response_url = None),
        ("wrong final URL", |value| {
            value.response_url = Some(URL.into())
        }),
        ("status", |value| value.status = 404),
        ("head time", |value| value.fetched_at = DONE.into()),
        ("timestamp alias", |value| {
            value.fetched_at = "2026-10-21T08:00:03Z".into()
        }),
        ("declared digest", |value| {
            value.content_digest = "c".repeat(64)
        }),
        ("declared bytes", |value| value.bytes += 1),
    ];
    for (field, mutate) in mutations {
        let mut changed = capture.clone();
        mutate(&mut changed);
        check!(
            observed(&bytes, &changed).is_err(),
            "changed {field} was accepted"
        );
        refused(&bytes, &changed)?;
    }
    check!(eq; observed(&bytes, &capture)?, Some(expected));
    Ok(())
}

#[test]
fn actual_body_tampering_refuses_even_if_declared_length_or_digest_is_repaired() -> TestResult {
    let (capture, expected) = fixture::original()?;
    let bytes = fixture::encoded(&expected)?;
    let mut changed = capture.clone();
    let byte = changed.body.first_mut().ok_or("source body")?;
    *byte ^= 1;
    check!(eq; changed.body.len(), capture.bytes);
    refused(&bytes, &changed)?;
    changed.content_digest = crate::net::cache::content_digest(&changed.body);
    refused(&bytes, &changed)?;
    let mut changed = capture.clone();
    changed.body.push(b'!');
    refused(&bytes, &changed)?;
    changed.bytes = changed.body.len();
    refused(&bytes, &changed)?;
    check!(eq; observed(&bytes, &capture)?, Some(expected));
    Ok(())
}

#[test]
fn valid_observation_for_another_body_cannot_bind_the_original_capture() -> TestResult {
    let (capture, expected) = fixture::original()?;
    let bytes = fixture::encoded(&expected)?;
    for (path, value) in [
        ("/observed_http/body_digest", json!("d".repeat(64))),
        ("/observed_http/body_bytes", json!(capture.bytes + 1)),
    ] {
        refused(&fixture::changed(&bytes, path, value)?, &capture)?;
    }
    Ok(())
}
