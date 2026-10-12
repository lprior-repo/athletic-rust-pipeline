use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_reconcile::identity::Revision;
use census_service::restate_services::{HistoryWindow, JurisdictionRequest};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

async fn fixture(listener: tokio::net::TcpListener) -> TestResult<Vec<String>> {
    let mut paths = Vec::new();
    for _ in 0..2 {
        let (socket, _) = listener.accept().await?;
        let mut socket = BufReader::new(socket);
        let mut request = String::new();
        socket.read_line(&mut request).await?;
        paths.push(request);
        let mut length = 0;
        for _ in 0..128 {
            let mut header = String::new();
            socket.read_line(&mut header).await?;
            if header == "\r\n" {
                break;
            }
            if let Some((key, value)) = header.split_once(':') {
                if key.eq_ignore_ascii_case("content-length") {
                    length = value.trim().parse::<usize>()?;
                }
            }
        }
        if length > 8192 {
            return Err("fixture request too large".into());
        }
        socket.read_exact(&mut vec![0; length]).await?;
        let body = "{\"message\":\"fixture acquisition refusal\"}";
        let response = format!(
            "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.get_mut().write_all(response.as_bytes()).await?;
        socket.get_mut().shutdown().await?;
    }
    Ok(paths)
}

fn requests() -> TestResult<Vec<JurisdictionRequest>> {
    [UsJurisdiction::Wisconsin, UsJurisdiction::Minnesota]
        .into_iter()
        .map(|jurisdiction| {
            Ok(JurisdictionRequest {
                jurisdiction,
                season: SchoolYear::new(2026).ok_or("fixture season")?,
                revision: Revision(1),
                history: HistoryWindow::cohort("2026-10-12")?,
                refresh: false,
                limit_per_state: None,
                concurrency: 1,
                observed_on: None,
                authorized_hosts: Vec::new(),
                source_parallelism: 1,
            })
        })
        .collect()
}

#[tokio::test]
async fn ingress_team_collection_finishes_every_independent_request_after_refusal() -> TestResult {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}/", listener.local_addr()?);
    let requests = requests()?;
    let result = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        tokio::join!(fixture(listener), super::drive(&origin, requests, 0, true))
    })
    .await?;
    let (paths, outcome) = result;
    let paths = paths?;
    check!(eq; paths.len(), 2);
    check!(paths.first().is_some_and(|path| path.contains("WI")));
    check!(paths.get(1).is_some_and(|path| path.contains("MN")));
    let error = outcome
        .err()
        .ok_or("refused ingress collection succeeded")?;
    let typed = error
        .downcast_ref::<super::super::TeamsCollectionError>()
        .ok_or("opaque aggregate")?;
    check!(eq; typed.incomplete, vec![UsJurisdiction::Wisconsin, UsJurisdiction::Minnesota]);
    check!(eq; typed.disposition, census_crawl::CollectionDisposition::Partial);
    Ok(())
}
