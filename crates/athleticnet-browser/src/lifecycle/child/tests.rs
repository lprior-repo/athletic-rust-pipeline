use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn owned_stderr_accepts_only_the_announced_loopback_browser_endpoint() -> TestResult {
    let endpoint = "ws://127.0.0.1:43871/devtools/browser/exact-child";
    check!(eq;
        parse_endpoint(format!("DevTools listening on {endpoint}\r").as_bytes())?,
        Some(url::Url::parse(endpoint)?)
    );
    check!(eq;
        parse_endpoint(b"unrelated stderr containing ws://127.0.0.1:9222/")?,
        None
    );
    for raw in [
        "ws://192.0.2.1:9222/devtools/browser/foreign",
        "ws://user@127.0.0.1:9222/devtools/browser/foreign",
        "ws://127.0.0.1:9222/devtools/page/foreign",
        "http://127.0.0.1:9222/devtools/browser/foreign",
        "ws://127.0.0.1:9222/devtools/browser/foreign?query=1",
    ] {
        check!(matches!(
            parse_endpoint(format!("DevTools listening on {raw}").as_bytes()),
            Err(BrowserStartupError::LaunchFailed(_))
        ));
    }
    Ok(())
}

#[test]
fn endpoint_line_limit_refuses_without_an_unbounded_line_allocation() -> TestResult {
    let mut reader = EndpointReader {
        stderr: None,
        line: [0; MAX_STDERR_LINE],
        length: 0,
        visited: 0,
        deadline: Instant::now(),
    };
    for _ in 0..MAX_STDERR_LINE {
        check!(eq; reader.push(b'x')?, None);
    }
    check!(matches!(
        reader.push(b'x'),
        Err(BrowserStartupError::LaunchFailed(_))
    ));
    check!(eq; reader.length, MAX_STDERR_LINE);
    Ok(())
}

#[test]
fn invalid_announced_utf8_is_an_attributable_startup_failure() -> TestResult {
    check!(matches!(
        parse_endpoint(b"DevTools listening on \xff"),
        Err(BrowserStartupError::LaunchFailed(_))
    ));
    Ok(())
}

#[cfg(unix)]
mod physical_regression;
