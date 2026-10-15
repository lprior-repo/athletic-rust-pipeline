use super::*;
use futures::{stream, TryStreamExt};
use http::{HeaderMap, StatusCode};
use std::sync::atomic::{AtomicUsize, Ordering};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn admission() -> Admission {
    Admission::new(false, StatusCode::OK, &HeaderMap::new()).with_limit(8)
}

#[tokio::test]
async fn streamed_body_admits_empty_and_exact_limit_without_trusting_small_declaration(
) -> TestResult {
    let bound = admission();
    let empty = stream::iter(Vec::<Result<Bytes, IngressError>>::new());
    check!(read(empty, &bound).await?.is_empty());
    bound.declared(Some(1))?;
    let parts: Vec<Result<Bytes, IngressError>> = vec![
        Ok(Bytes::from_static(b"123")),
        Ok(Bytes::from_static(b"45678")),
    ];
    check!(eq; read(stream::iter(parts), &bound).await?.as_ref(), b"12345678");
    Ok(())
}

#[tokio::test]
async fn streamed_overflow_refuses_before_polling_any_remaining_chunks() -> TestResult {
    let polled = AtomicUsize::new(0);
    let parts: Vec<Result<Bytes, IngressError>> = vec![
        Ok(Bytes::from_static(b"123456789")),
        Ok(Bytes::from_static(b"must not be consumed")),
    ];
    let stream = stream::iter(parts).inspect_ok(|_| {
        polled.fetch_add(1, Ordering::SeqCst);
    });
    let error = read(stream, &admission())
        .await
        .err()
        .ok_or("oversized streamed body accepted")?;
    check!(matches!(
        error,
        IngressError::Refused(super::super::ResponseRefusal {
            kind: AdmissionKind::StreamedBodyBytes,
            limit: 8,
            observed: 9,
            ..
        })
    ));
    check!(eq; polled.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test]
async fn streamed_transport_failure_never_returns_a_partial_body() -> TestResult {
    let source = reqwest::Client::builder()
        .user_agent("\n")
        .build()
        .err()
        .ok_or("invalid header accepted")?;
    let parts = vec![
        Ok(Bytes::from_static(b"valid")),
        Err(IngressError::Transport(source)),
    ];
    let error = read(stream::iter(parts), &admission())
        .await
        .err()
        .ok_or("partial body returned")?;
    check!(matches!(error, IngressError::Transport(source) if source.is_builder()));
    Ok(())
}

#[tokio::test]
async fn empty_chunks_have_exact_finite_admission_independent_of_body_bytes() -> TestResult {
    let bound = admission().with_empty_limit(2);
    let exact: Vec<Result<Bytes, IngressError>> = vec![
        Ok(Bytes::new()),
        Ok(Bytes::from_static(b"12345678")),
        Ok(Bytes::new()),
    ];
    check!(eq; read(stream::iter(exact), &bound).await?.as_ref(), b"12345678");
    let polled = AtomicUsize::new(0);
    let excessive: Vec<Result<Bytes, IngressError>> = vec![
        Ok(Bytes::new()),
        Ok(Bytes::new()),
        Ok(Bytes::new()),
        Ok(Bytes::from_static(b"unread")),
    ];
    let stream = stream::iter(excessive).inspect_ok(|_| {
        polled.fetch_add(1, Ordering::SeqCst);
    });
    let error = read(stream, &bound)
        .await
        .err()
        .ok_or("empty chunk excess admitted")?;
    check!(matches!(
        error,
        IngressError::Refused(super::super::ResponseRefusal {
            kind: AdmissionKind::EmptyChunks,
            limit: 2,
            observed: 3,
            ..
        })
    ));
    check!(eq; polled.load(Ordering::SeqCst), 3);
    Ok(())
}
