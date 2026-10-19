use super::*;
use clap::Parser;

type Exchange = (Vec<(String, Vec<u8>)>, anyhow::Result<()>);

async fn exchange(
    flags: &[&str],
    responses: Vec<(&str, String)>,
) -> Result<Exchange, Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}/", listener.local_addr()?);
    let cli = crate::cli::Cli::try_parse_from(
        [
            "census-service",
            "national",
            "--states",
            "WI",
            "--revision",
            "1",
            "--ingress",
            &origin,
            "--detach",
        ]
        .into_iter()
        .chain(flags.iter().copied()),
    )?;
    let crate::cli::command::Command::National(args) = &cli.command else {
        return Err("authority case parsed as another command".into());
    };
    let (finished, completion) = tokio::sync::oneshot::channel();
    let (captured, outcome) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(
            guarded_resume_ingress(listener, responses, completion),
            async {
                let outcome = run_national(&cli, args).await;
                let _sent = finished.send(());
                outcome
            }
        )
    })
    .await?;
    Ok((captured?, outcome))
}

fn require_attachment_refusal(exchange: Exchange) -> Result<(), Box<dyn std::error::Error>> {
    let (captured, outcome) = exchange;
    check!(outcome.is_err());
    check!(eq; captured.len(), 3);
    check!(captured[0]
        .0
        .contains("/Census/publication_capabilities HTTP/1.1"));
    check!(captured[1].0.contains("/run/send HTTP/1.1"));
    check!(captured[2].0.contains("/request HTTP/1.1"));
    Ok(())
}

#[tokio::test]
async fn explicit_resume_generation_mismatch_refuses_before_submission(
) -> Result<(), Box<dyn std::error::Error>> {
    let (captured, outcome) = exchange(
        &["--resume", "7", "--generation", "qualified-v1"],
        vec![
            ("200 OK", bound_request(&["WI"])?),
            ("200 OK", bound_history(Some("2026-10-01"))?),
        ],
    )
    .await?;
    check!(outcome.is_err());
    check!(eq; captured.len(), 2);
    check!(captured[0].0.contains("/request HTTP/1.1"));
    check!(captured[1].0.contains("/state HTTP/1.1"));
    Ok(())
}

#[tokio::test]
async fn previously_accepted_profile_mismatch_refuses_before_attachment(
) -> Result<(), Box<dyn std::error::Error>> {
    let store = "a".repeat(64);
    let profile = "c".repeat(64);
    require_attachment_refusal(
        exchange(
            &[
                "--selected-store-identity",
                &store,
                "--selected-profile-id",
                &profile,
            ],
            vec![
                ("200 OK", capabilities()?),
                (
                    "202 Accepted",
                    serde_json::json!({
                        "invocationId": "inv_1aiqX0vFEFNH1Umgre58JiCLgHfTtztYK5",
                        "status": "PreviouslyAccepted"
                    })
                    .to_string(),
                ),
                ("200 OK", bound_request(&["WI"])?),
            ],
        )
        .await?,
    )
}

#[tokio::test]
async fn failed_submission_store_mismatch_refuses_before_attachment(
) -> Result<(), Box<dyn std::error::Error>> {
    let store = "c".repeat(64);
    let profile = "b".repeat(64);
    require_attachment_refusal(
        exchange(
            &[
                "--selected-store-identity",
                &store,
                "--selected-profile-id",
                &profile,
            ],
            vec![
                ("200 OK", capabilities()?),
                (
                    "409 Conflict",
                    serde_json::json!({"message": "existing invocation"}).to_string(),
                ),
                ("200 OK", bound_request(&["WI"])?),
            ],
        )
        .await?,
    )
}
