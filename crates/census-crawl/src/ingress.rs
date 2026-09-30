use bytes::Bytes;
use http::{Request as HttpRequest, Response as HttpResponse, Uri};
use restate_sdk::ingress::{Client, ClientBuildError, RequestExecutor};

const SDK_ROUTE_PREFIX: &str = "/restate";

const SDK_SYNC_OPERATIONS: [&str; 2] = ["call", "invoke"];

const SDK_SEND_OPERATION: &str = "send";

pub type Ingress = Client<CurrentRoute>;

#[derive(Clone)]
pub struct CurrentRoute {
    transport: reqwest::Client,
}

impl CurrentRoute {
    pub fn over(transport: reqwest::Client) -> Self {
        Self { transport }
    }
}

impl RequestExecutor for CurrentRoute {
    type Error = reqwest::Error;

    async fn execute(
        &self,
        mut request: HttpRequest<Bytes>,
    ) -> Result<HttpResponse<Bytes>, Self::Error> {
        if let Some(uri) = invocation_route(request.uri()) {
            *request.uri_mut() = uri;
        }
        <reqwest::Client as RequestExecutor>::execute(&self.transport, request).await
    }
}

pub fn client(origin: Uri, transport: reqwest::Client) -> Result<Ingress, ClientBuildError> {
    Client::new(origin, CurrentRoute::over(transport))
}

fn invocation_route(uri: &Uri) -> Option<Uri> {
    let path = uri.path();
    let remainder = path.strip_prefix(SDK_ROUTE_PREFIX)?.strip_prefix('/')?;
    let invoked = match SDK_SYNC_OPERATIONS
        .iter()
        .find_map(|operation| remainder.strip_prefix(operation))
        .filter(|invoked| invoked.starts_with('/'))
    {
        Some(invoked) => invoked.to_string(),
        None => {
            let sent = remainder
                .strip_prefix(SDK_SEND_OPERATION)?
                .strip_prefix('/')?;
            format!("/{sent}/send")
        }
    };
    let path_and_query = match uri.query() {
        Some(query) => format!("{invoked}?{query}"),
        None => invoked,
    };
    let mut parts = uri.clone().into_parts();
    parts.path_and_query = Some(path_and_query.parse().ok()?);
    Uri::from_parts(parts).ok()
}

#[cfg(test)]
mod tests {
    use super::invocation_route;
    use http::Uri;

    fn rewritten(value: &str) -> Option<String> {
        value
            .parse::<Uri>()
            .ok()
            .and_then(|uri| invocation_route(&uri))
            .map(|uri| uri.to_string())
    }

    #[test]
    fn rewrites_sync_call_route_to_served_path() {
        assert_eq!(
            rewritten("http://127.0.0.1:18095/restate/call/Census/open_work").as_deref(),
            Some("http://127.0.0.1:18095/Census/open_work")
        );
        assert_eq!(
            rewritten("http://127.0.0.1:18095/restate/call/BrowserSession/profile-0/fetch")
                .as_deref(),
            Some("http://127.0.0.1:18095/BrowserSession/profile-0/fetch")
        );
    }

    #[test]
    fn rewrites_legacy_invoke_route() {
        assert_eq!(
            rewritten("http://127.0.0.1:18095/restate/invoke/Report/report").as_deref(),
            Some("http://127.0.0.1:18095/Report/report")
        );
    }

    #[test]
    fn keeps_query_and_authority() {
        assert_eq!(
            rewritten("https://census.test:8443/restate/call/Census/status?delay=PT1S").as_deref(),
            Some("https://census.test:8443/Census/status?delay=PT1S")
        );
    }

    #[test]
    fn rewrites_send_route_to_the_served_suffix_form() {
        assert_eq!(
            rewritten("http://127.0.0.1:18095/restate/send/Census/open_work").as_deref(),
            Some("http://127.0.0.1:18095/Census/open_work/send")
        );
        assert_eq!(
            rewritten(
                "http://127.0.0.1:18095/restate/send/NationalCensus/national:2026-27:aa:11/run"
            )
            .as_deref(),
            Some("http://127.0.0.1:18095/NationalCensus/national:2026-27:aa:11/run/send")
        );
        assert_eq!(
            rewritten("https://census.test:8443/restate/send/Report/report?delay=PT1S").as_deref(),
            Some("https://census.test:8443/Report/report/send?delay=PT1S")
        );
    }

    #[test]
    fn leaves_unserved_routes_untouched() {
        for value in [
            "http://127.0.0.1:18095/restate/output/inv_1",
            "http://127.0.0.1:18095/restate/attach/inv_1",
            "http://127.0.0.1:18095/restate/scope/tenant/call/Census/report",
            "http://127.0.0.1:18095/restate/health",
            "http://127.0.0.1:18095/restate/invocation-probe",
            "http://127.0.0.1:18095/restate/call",
            "http://127.0.0.1:18095/restate/send",
            "http://127.0.0.1:18095/restate/sent/Report",
            "http://127.0.0.1:18095/restate/called/Report",
        ] {
            assert_eq!(rewritten(value), None, "{value} must not be rewritten");
        }
    }
}
