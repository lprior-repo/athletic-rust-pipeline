use super::super::collect::collect_directory;
use super::support::*;
use census_store::Table;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[test]
fn http_errors_and_disconnected_transport_remain_unfinished(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            for status in [Some(404), Some(403), Some(500), None] {
                let (_root, store, _) = setup()?;
                let fetcher = fetcher(&store)?;
                let listener = TcpListener::bind("127.0.0.1:0").await?;
                let url = format!("http://{}/Directory.aspx", listener.local_addr()?);
                let ctx = context(&fetcher, &store, None, "2026-10-02")?;
                let client = async {
                    let report = collect_directory(&ctx, &Default::default(), &url).await?;
                    check!(eq; (report.rows, report.errors, report.from_cache), (0, 1, 0));
                    if let Some(status) = status {
                        check!(report
                            .notes
                            .iter()
                            .any(|note| note.contains(&status.to_string())));
                    } else {
                        check!(report
                            .notes
                            .iter()
                            .any(|note| note.contains("transport error")));
                    }
                    check!(eq; physical_counts(&store)?, vec![0, 0, 0]);
                    check!(eq; store.journal_keys("riil_schools")?.len(), 0);
                    Ok::<_, Box<dyn std::error::Error>>(())
                };
                tokio::time::timeout(Duration::from_secs(10), async {
                    tokio::try_join!(serve(listener, status, b"Unavailable"), client)
                })
                .await??;
            }
            Ok(())
        })
}

#[test]
fn malformed_or_refused_captures_cannot_create_completion_markers(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    let malformed = DIRECTORY.replace("<td>Morgan Coach</td>", "<td>Morgan Coach");
    for body in [
        b"".as_slice(),
        b"<html><b>Access denied</b></html>".as_slice(),
        b"<div id='BodyWrapper'><ul><li>Access denied</li></ul></div>".as_slice(),
        b"<details><summary>Example HS</summary>".as_slice(),
        b"<details><summary></summary><table></table></details>".as_slice(),
        b"<details><summary>Example HS</summary><table></details>".as_slice(),
        b"<details><summary>Example HS</summary><table><tr><td>Boys Cross Country</td><td>Head Coach</td><td>Alex Coach</td></tr></table></details>".as_slice(),
        malformed.as_bytes(),
        b"\xff\xfe".as_slice(),
    ] {
        let (_root, store, fetcher) = setup()?;
        seed(&fetcher, body, ACQUIRED_AT)?;
        let report = evaluate(&fetcher, &store, None, "2026-10-02", "2026-10-03").await?;
        check!(report.unfinished.iter().any(|locator| locator.starts_with(URL)));
        let schools = store.scan::<census_domain::model::CanonicalSchool>(Table::Schools)?;
        let owned = std::str::from_utf8(body).is_ok_and(|text| text.contains("<summary>Example HS</summary>"));
        check!(eq; schools.iter().map(|school| school.name.as_str()).collect::<std::collections::BTreeSet<_>>(),
            if owned { std::collections::BTreeSet::from(["Example HS"]) } else { std::collections::BTreeSet::new() });
        let coaches = store.scan::<census_domain::model::CanonicalCoach>(Table::Coaches)?;
        check!(eq; coaches.iter().map(|coach| coach.name.as_str()).collect::<std::collections::BTreeSet<_>>(),
            if body == malformed.as_bytes() { std::collections::BTreeSet::from(["Alex Coach"]) } else { std::collections::BTreeSet::new() });
        check!(eq; store.journal_keys("riil_schools")?.len(), 0);
    }
    Ok(())
    })
}

#[test]
fn refused_new_content_is_visible_after_an_earlier_committed_capture(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_root, store, fetcher) = setup()?;
            seed(&fetcher, DIRECTORY.as_bytes(), ACQUIRED_AT)?;
            check!(eq;
                evaluate(&fetcher, &store, None, "2026-10-02", "")
                    .await?
                    .rows,
                1
            );
            seed(
                &fetcher,
                b"<html><title>Access Denied</title></html>",
                "2026-10-01T11:00:00Z",
            )?;
            let report = evaluate(&fetcher, &store, None, "2026-10-03", "").await?;
            check!(eq; (report.rows, report.errors), (0, 1));
            check!(eq; physical_counts(&store)?, vec![1, 2, 1]);
            check!(eq; store.journal_keys("riil_schools")?.len(), 1);
            Ok(())
        })
}

#[test]
fn published_empty_directory_and_vacant_appointments_are_not_fetch_failures(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let vacancy = DIRECTORY
                .replace("Alex Coach", "")
                .replace("Morgan Coach", "");
            for (body, expected) in [
                ("<div id=\"BodyWrapper\"><ul></ul></div>", vec![0, 0, 0]),
                (vacancy.as_str(), vec![1, 0, 1]),
            ] {
                let (_root, store, fetcher) = setup()?;
                seed(&fetcher, body.as_bytes(), ACQUIRED_AT)?;
                let report = evaluate(&fetcher, &store, None, "2026-10-02", "").await?;
                check!(eq; report.errors, 0);
                check!(eq; physical_counts(&store)?, expected);
                check!(eq; report.rows, *expected.first().ok_or("expected school count")?);
            }
            Ok(())
        })
}

#[test]
fn authentic_directory_capture_retains_school_selection() -> Result<(), Box<dyn std::error::Error>>
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_root, store, fetcher) = setup()?;
            seed(&fetcher, super::FIXTURE.as_bytes(), ACQUIRED_AT)?;
            let report = evaluate(&fetcher, &store, None, "2026-10-02", "").await?;
            check!(eq; report.errors, 0);
            let schools =
                observations::<census_domain::model::CanonicalSchool>(&store, Table::Schools)?;
            check!(schools.iter().any(|school| school.name == "Barrington HS"));
            check!(schools
                .iter()
                .any(|school| school.name.contains("Bishop Hendricken")));
            Ok(())
        })
}

#[test]
fn http_200_refusal_is_not_a_complete_empty_directory() -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_root, store, _) = setup()?;
            let fetcher = fetcher(&store)?;
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let url = format!("http://{}/Directory.aspx", listener.local_addr()?);
            let ctx = context(&fetcher, &store, None, "2026-10-02")?;
            let client = async {
                let report = collect_directory(&ctx, &Default::default(), &url).await?;
                check!(eq; (report.rows, report.errors, report.from_cache), (0, 1, 0));
                let replay = collect_directory(&ctx, &Default::default(), &url).await?;
                check!(eq; (replay.rows, replay.errors, replay.from_cache), (0, 1, 1));
                check!(eq; physical_counts(&store)?, vec![0, 0, 0]);
                check!(eq; store.journal_keys("riil_schools")?.len(), 0);
                Ok::<_, Box<dyn std::error::Error>>(())
            };
            tokio::time::timeout(Duration::from_secs(10), async {
                tokio::try_join!(
                    serve(
                        listener,
                        Some(200),
                        b"<html><title>Access Denied</title></html>"
                    ),
                    client
                )
            })
            .await??;
            Ok(())
        })
}

async fn serve(
    listener: TcpListener,
    status: Option<u16>,
    body: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    for (path, status, body) in [("/Directory.aspx", status, body)] {
        let (mut socket, _) = listener.accept().await?;
        let request = read_request(&mut socket).await?;
        check!(eq; request.split_whitespace().nth(1), Some(path));
        if let Some(status) = status {
            let header = format!(
                "HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            socket.write_all(header.as_bytes()).await?;
            socket.write_all(body).await?;
        }
    }
    Ok(())
}

async fn read_request(socket: &mut tokio::net::TcpStream) -> anyhow::Result<String> {
    let mut request = Vec::new();
    for _ in 0..16 {
        let mut bytes = [0; 4096];
        let count = socket.read(&mut bytes).await?;
        if count == 0 {
            break;
        }
        request.extend_from_slice(&bytes[..count]);
        if request.windows(4).any(|part| part == b"\r\n\r\n") {
            return Ok(String::from_utf8(request)?);
        }
    }
    anyhow::bail!("fixture request exceeds its bounded header read");
}
