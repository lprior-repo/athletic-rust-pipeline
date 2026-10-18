use super::*;

#[tokio::test]
async fn final_report_contact_notice_reads_exact_base_or_resume_without_new_submission(
) -> TestResult {
    let partial = serde_json::json!([{
        "jurisdiction": "WI", "identity": "retained-contact-workflow", "status": "partial",
        "source_rows": 0, "errors": 1, "unfinished": 1
    }])
    .to_string();
    for response in [
        ("200 OK", partial),
        ("200 OK", "null".into()),
        absent(),
        ("200 OK", "malformed contact report".into()),
    ] {
        for resumed in [false, true] {
            let (arguments, mut responses) = if resumed {
                (
                    vec!["--drive", "--resume", "7", "--json"],
                    selected_prefix()?,
                )
            } else {
                (
                    vec!["--drive", "--json"],
                    vec![("200 OK", capabilities()?), accepted()],
                )
            };
            responses.push(ready(&complete_report()?)?);
            responses.push(response.clone());
            let (captured, outcome) = exchange_mode(&arguments, responses, false).await?;
            outcome?;
            check!(eq; captured.len(), if resumed { 6 } else { 4 });
            let sends = send_paths(&captured);
            check!(eq; sends.len(), 1);
            let sent = sends.first().ok_or("submission absent")?;
            let contact_path = sent.replace("/run/send ", "/contact_report ");
            check!(captured
                .last()
                .is_some_and(|(path, _)| *path == contact_path));
        }
    }
    Ok(())
}

#[tokio::test]
async fn final_report_contact_notice_timeout_preserves_success_without_resubmission() -> TestResult
{
    let responses = vec![
        ("200 OK", capabilities()?),
        accepted(),
        ready(&complete_report()?)?,
        ("200 OK", String::new()),
    ];
    let (captured, outcome) =
        exchange_with_held_reply(&["--drive"], responses, false, Some((3, 4096))).await?;
    outcome?;
    check!(eq; captured.len(), 4);
    check!(eq; send_paths(&captured).len(), 1);
    check!(captured
        .last()
        .is_some_and(|(path, _)| path.contains("/contact_report ")));
    Ok(())
}
