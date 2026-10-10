use super::super::super::request_headers;
use super::super::{pinned_fetcher_with, TestResult};
use crate::net::{FetchError, FetchOptions};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

#[derive(Clone, Copy)]
enum OriginChange {
    Host,
    Port,
    Scheme,
}

async fn observe_connections(
    listener: TcpListener,
    mut finished: oneshot::Receiver<()>,
) -> TestResult<usize> {
    let mut connections = 0;
    for _ in 0..8 {
        tokio::select! {
            biased;
            accepted = listener.accept() => {
                let (socket, _) = accepted?;
                connections += 1;
                drop(socket);
            }
            completed = &mut finished => {
                completed?;
                return Ok(connections);
            }
        }
    }
    Err("refused destination exceeded connection bound".into())
}

async fn serve_origin(
    listener: TcpListener,
    destination: &str,
    finished: oneshot::Receiver<()>,
) -> TestResult<(Vec<String>, usize)> {
    let mut paths = Vec::new();
    for _ in 0..2 {
        let (mut socket, _) = listener.accept().await?;
        let headers = request_headers(&mut socket).await?;
        let path = headers
            .split_whitespace()
            .nth(1)
            .ok_or("missing request path")?;
        let response = match path {
            "/start" => format!(
                "HTTP/1.1 302 Found\r\nLocation: {destination}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            ),
            other => return Err(format!("unexpected origin request: {other}").into()),
        };
        paths.push(path.to_string());
        socket.write_all(response.as_bytes()).await?;
    }
    Ok((paths, observe_connections(listener, finished).await?))
}

async fn assert_origin_change_is_undispatched(change: OriginChange) -> TestResult {
    let root = tempfile::tempdir()?;
    let source = TcpListener::bind("127.0.0.1:0").await?;
    let target = TcpListener::bind("127.0.0.1:0").await?;
    let address = source.local_addr()?;
    let target_address = target.local_addr()?;
    let requested = format!("http://source.example:{}/start", address.port());
    let destination = match change {
        OriginChange::Host => format!("http://other.example:{}/destination", address.port()),
        OriginChange::Port => format!(
            "http://source.example:{}/destination",
            target_address.port()
        ),
        OriginChange::Scheme => format!("https://source.example:{}/destination", address.port()),
    };
    let fetcher = pinned_fetcher_with(
        root.path(),
        &[("source.example", address), ("other.example", address)],
        Vec::new(),
    )?;
    let (source_finished, source_completion) = oneshot::channel();
    let (target_finished, target_completion) = oneshot::channel();
    let acquisition = async {
        let outcome = fetcher.get(&requested, &FetchOptions::default()).await;
        source_finished
            .send(())
            .map_err(|()| "origin server stopped early")?;
        target_finished
            .send(())
            .map_err(|()| "target observer stopped early")?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(outcome)
    };
    let ((paths, source_connections), target_connections, outcome) =
        tokio::time::timeout(Duration::from_secs(10), async {
            tokio::try_join!(
                serve_origin(source, &destination, source_completion),
                observe_connections(target, target_completion),
                acquisition
            )
        })
        .await??;
    check!(eq; paths, ["/start"]);
    check!(eq;
        source_connections, 0,
        "redirect contacted original listener"
    );
    check!(eq;
        target_connections, 0,
        "redirect contacted changed-port listener"
    );
    match outcome {
        Err(FetchError::Policy { detail }) => {
            check!(
                detail.contains("bypasses admission"),
                "the refusal names the admission boundary: {detail}"
            );
        }
        other => return Err(format!("expected an admission refusal, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn empty_grants_cross_host_redirect_never_contacts_destination() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { assert_origin_change_is_undispatched(OriginChange::Host).await })
}

#[test]
fn empty_grants_same_host_changed_port_redirect_never_contacts_destination() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { assert_origin_change_is_undispatched(OriginChange::Port).await })
}

#[test]
fn empty_grants_same_host_changed_scheme_redirect_never_contacts_destination() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { assert_origin_change_is_undispatched(OriginChange::Scheme).await })
}
