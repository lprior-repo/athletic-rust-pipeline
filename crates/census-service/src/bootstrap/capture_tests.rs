use super::*;
use crate::bootstrap::ServeCaptureProtocol;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn capture_protocol_flag_has_explicit_closed_modes() -> TestResult {
    check!(eq; ServeOptions::default().capture_protocol, ServeCaptureProtocol::LegacyFresh);
    let options = ServeOptions::from_env(
        ["--capture-protocol", "fresh-qualification-v1"]
            .into_iter()
            .map(str::to_string),
    )?;
    check!(eq; options.capture_protocol, ServeCaptureProtocol::FreshQualificationV1);
    check!(matches!(
        ServeOptions::from_env(
            ["--capture-protocol", "future"]
                .into_iter()
                .map(str::to_string)
        ),
        Err(BootstrapError::CaptureProtocolUnsupported { .. })
    ));
    check!(ServeOptions::from_env(["--capture-protocol".to_string()].into_iter()).is_err());
    Ok(())
}

#[tokio::test]
async fn capture_default_serve_refuses_before_listener_and_explicit_serve_admits() -> TestResult {
    let root = tempfile::tempdir()?;
    let qualification = Store::create_capture_qualification_v1(root.path())?;
    let occupied = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
    let options = ServeOptions {
        data_dir: root.path().to_path_buf(),
        listen: occupied.local_addr()?,
        ..ServeOptions::default()
    };
    let refused = serve_until(options.clone(), std::future::ready(())).await;
    check!(refused.is_err());
    let error = refused.err().ok_or("missing refusal")?;
    check!(error.to_string().contains("explicit opt-in"));
    let options = ServeOptions {
        capture_protocol: ServeCaptureProtocol::FreshQualificationV1,
        listen: "127.0.0.1:0".parse()?,
        ..options
    };
    let drained = serve_until(options, std::future::ready(())).await?;
    check!(eq; drained.panicked, 0);
    check!(eq; Store::capture_qualification_control_existing(root.path())?.show()?, qualification);
    Ok(())
}

#[tokio::test]
async fn capture_explicit_serve_never_initializes_or_adopts_a_root() -> TestResult {
    let parent = tempfile::tempdir()?;
    let absent = parent.path().join("absent");
    let options = ServeOptions {
        data_dir: absent.clone(),
        capture_protocol: ServeCaptureProtocol::FreshQualificationV1,
        ..ServeOptions::default()
    };
    check!(serve_until(options, std::future::ready(())).await.is_err());
    check!(!absent.exists());
    let legacy = parent.path().join("legacy");
    drop(Store::open(&legacy)?);
    let before = Store::format(&legacy)?;
    let options = ServeOptions {
        data_dir: legacy.clone(),
        capture_protocol: ServeCaptureProtocol::FreshQualificationV1,
        ..ServeOptions::default()
    };
    check!(serve_until(options, std::future::ready(())).await.is_err());
    check!(eq; Store::format(&legacy)?, before);
    Ok(())
}
