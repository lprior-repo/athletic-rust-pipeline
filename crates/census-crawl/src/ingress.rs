use bytes::Bytes;
use futures::TryStreamExt;
use http::{Request as HttpRequest, Response as HttpResponse, Uri};
use restate_sdk::ingress::{Client, ClientBuildError, RequestExecutor};

mod body;
mod budget;
mod error;
pub use budget::ResponseClass;
pub use error::{is_admission_error, AdmissionKind, IngressError, ResponseRefusal};

const SDK_ROUTE_PREFIX: &str = "/restate";

const SDK_OUTPUT_OPERATION: &str = "output/";

const SDK_ATTACH_OPERATION: &str = "attach/";

const SERVED_INVOCATION_ROOT: &str = "/restate/invocation";

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
    type Error = IngressError;

    #[tracing::instrument(skip_all)]
    async fn execute(
        &self,
        mut request: HttpRequest<Bytes>,
    ) -> Result<HttpResponse<Bytes>, Self::Error> {
        if let Some(uri) = invocation_route(request.uri()) {
            *request.uri_mut() = uri;
        }
        let control = budget::is_control(request.method(), request.uri());
        let requested_invocation = budget::requested_invocation(request.uri());
        let request: reqwest::Request = request.try_into()?;
        let response = self.transport.execute(request).await.map_err(|source| {
            error::response_error(source, requested_invocation.as_deref(), None)
        })?;
        let admission = budget::Admission::new(control, response.status(), response.headers())
            .with_requested_invocation(requested_invocation);
        admission.headers(response.headers())?;
        admission.declared(response.content_length())?;
        let status = response.status();
        let version = response.version();
        let headers = response.headers().clone();
        let body = body::read(
            response
                .bytes_stream()
                .map_err(|source| admission.transport_error(source)),
            &admission,
        )
        .await?;
        let mut response = HttpResponse::new(body);
        *response.status_mut() = status;
        *response.version_mut() = version;
        *response.headers_mut() = headers;
        Ok(response)
    }
}

pub fn client(origin: Uri, transport: reqwest::Client) -> Result<Ingress, ClientBuildError> {
    Client::new(origin, CurrentRoute::over(transport))
}

fn invocation_route(uri: &Uri) -> Option<Uri> {
    let path = uri.path();
    let remainder = path.strip_prefix(SDK_ROUTE_PREFIX)?.strip_prefix('/')?;
    let invoked = if let Some(invocation) = remainder.strip_prefix(SDK_OUTPUT_OPERATION) {
        format!("{SERVED_INVOCATION_ROOT}/{invocation}/output")
    } else if let Some(invocation) = remainder.strip_prefix(SDK_ATTACH_OPERATION) {
        format!("{SERVED_INVOCATION_ROOT}/{invocation}/attach")
    } else if let Some(invoked) = SDK_SYNC_OPERATIONS
        .iter()
        .find_map(|operation| remainder.strip_prefix(operation))
        .filter(|invoked| invoked.starts_with('/'))
    {
        invoked.to_string()
    } else {
        let sent = remainder
            .strip_prefix(SDK_SEND_OPERATION)?
            .strip_prefix('/')?;
        format!("/{sent}/send")
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
    fn rewrites_invocation_output_and_attach_routes_to_served_paths() {
        assert_eq!(
            rewritten("http://127.0.0.1:18095/restate/output/inv_1").as_deref(),
            Some("http://127.0.0.1:18095/restate/invocation/inv_1/output")
        );
        assert_eq!(
            rewritten("http://127.0.0.1:18095/restate/attach/inv_1").as_deref(),
            Some("http://127.0.0.1:18095/restate/invocation/inv_1/attach")
        );
    }

    #[test]
    fn leaves_unserved_routes_untouched() {
        for value in [
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

#[cfg(test)]
#[path = "ingress/transport_tests.rs"]
mod transport_tests;
