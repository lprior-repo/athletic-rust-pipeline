#![forbid(unsafe_code)]

//! Operator commands must relay the ingress's own failure message. Restate reports a
//! terminal handler failure as a JSON body on a 500 response, so a bare status code
//! hides why a submission was rejected.

use std::{
    io::{Read, Write},
    net::TcpListener,
    process::Command,
};

const FAILURE: &str = r#"{"code":500,"message":"requested concurrency exceeds worker capacity","source":"invocation"}"#;

fn failing_ingress() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("binding a loopback ingress");
    let port = listener
        .local_addr()
        .expect("reading the fake ingress address")
        .port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let mut request = [0_u8; 8192];
            let _ = stream.read(&mut request);
            let response = format!(
                "HTTP/1.1 500 Internal Server Error\r\n\
                 content-type: application/json\r\n\
                 x-restate-id: inv_1fsg95kbGEBB76fFdYt3O4GIZ84AQF6FXO\r\n\
                 content-length: {}\r\n\
                 connection: close\r\n\r\n{FAILURE}",
                FAILURE.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
    format!("http://127.0.0.1:{port}/")
}

#[test]
fn status_reports_the_ingress_failure_message() {
    let output = Command::new(env!("CARGO_BIN_EXE_athletic-rust-pipeline"))
        .args([
            "status",
            "--ingress",
            &failing_ingress(),
            "--run",
            &"a".repeat(64),
        ])
        .output()
        .expect("running the operator CLI");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "unexpected success: {stderr}");
    assert!(
        stderr.contains("requested concurrency exceeds worker capacity"),
        "the ingress failure message went unreported: {stderr}"
    );
}
