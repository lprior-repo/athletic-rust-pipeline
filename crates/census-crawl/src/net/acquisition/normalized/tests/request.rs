use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn refused(native: &reqwest::Request) -> TestResult {
    check!(matches!(
        super::super::request(native, START),
        Err(FetchError::CaptureDomain(CaptureDomainError::Invalid {
            field: "observed request method or body"
        }))
    ));
    Ok(())
}

#[test]
fn only_get_is_observed_even_when_other_methods_have_no_body() -> TestResult {
    for method in [
        reqwest::Method::POST,
        reqwest::Method::HEAD,
        reqwest::Method::PUT,
        reqwest::Method::DELETE,
        reqwest::Method::OPTIONS,
        reqwest::Method::CONNECT,
        reqwest::Method::TRACE,
        reqwest::Method::from_bytes(b"SOURCE")?,
    ] {
        let mut native = built_request(duplicates()?)?;
        *native.method_mut() = method.clone();
        refused(&native)?;
        check!(eq; native.method(), &method);
        check!(native.body().is_none());
        check!(eq; native.headers().get_all("x-evidence").iter().count(), 3);
        check!(eq; native.url().as_str(), URL);
    }
    Ok(())
}

#[test]
fn get_accepts_absent_or_explicit_empty_bytes_and_refuses_nonempty_bytes() -> TestResult {
    let mut native = built_request(duplicates()?)?;
    let absent = super::super::request(&native, START)?;
    *native.body_mut() = Some(reqwest::Body::from(Vec::<u8>::new()));
    let empty = super::super::request(&native, START)?;
    check!(eq; absent, empty);
    check!(eq; native.body().and_then(reqwest::Body::as_bytes), Some(b"".as_slice()));
    for bytes in [b"x".as_slice(), &[0xff, 0x00]] {
        *native.body_mut() = Some(reqwest::Body::from(bytes.to_vec()));
        refused(&native)?;
        check!(eq; native.body().and_then(reqwest::Body::as_bytes), Some(bytes));
    }
    Ok(())
}

#[test]
fn streaming_get_body_is_refused_without_polling_even_if_it_would_be_empty() -> TestResult {
    let polls = Arc::new(AtomicUsize::new(0));
    let observed_polls = Arc::clone(&polls);
    let stream = futures::stream::poll_fn(move |_| {
        observed_polls.fetch_add(1, Ordering::SeqCst);
        std::task::Poll::Ready(None::<Result<bytes::Bytes, std::io::Error>>)
    });
    let mut native = built_request(HeaderMap::new())?;
    *native.body_mut() = Some(reqwest::Body::wrap_stream(stream));
    check!(native.body().is_some());
    check!(native.body().and_then(reqwest::Body::as_bytes).is_none());
    refused(&native)?;
    check!(eq; polls.load(Ordering::SeqCst), 0);
    check!(native.body().is_some());
    Ok(())
}
