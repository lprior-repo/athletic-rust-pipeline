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
    check!(eq; Store::capture_qualification_control_existing(root.path())?.show()?, census_domain::model::CaptureQualification::V1(qualification));
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

#[test]
fn capture_v2_flag_is_explicit_and_does_not_change_default() -> TestResult {
    let options = ServeOptions::from_env(
        ["--capture-protocol", "fresh-qualification-v2"]
            .into_iter()
            .map(str::to_string),
    )?;
    check!(eq; options.capture_protocol, ServeCaptureProtocol::FreshQualificationV2);
    check!(eq; ServeOptions::default().capture_protocol, ServeCaptureProtocol::LegacyFresh);
    Ok(())
}

#[tokio::test]
async fn capture_v2_serve_requires_matching_explicit_protocol_before_listener() -> TestResult {
    let parent = tempfile::tempdir()?;
    let path = parent.path().join("fresh-v2");
    let qualification = Store::create_capture_qualification_v2(&path)?;
    let occupied = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
    let options = ServeOptions {
        data_dir: path.clone(),
        listen: occupied.local_addr()?,
        capture_protocol: ServeCaptureProtocol::FreshQualificationV1,
        ..ServeOptions::default()
    };
    let error = serve_until(options.clone(), std::future::ready(()))
        .await
        .err()
        .ok_or("v1 opener admitted v2")?
        .downcast::<BootstrapError>()?;
    let BootstrapError::StoreOpen { source, .. } = error else {
        return Err("mismatched protocol did not refuse at Store open".into());
    };
    check!(matches!(
        *source,
        census_store::StoreError::Capture(census_store::CaptureError::CaptureProtocolUnsupported)
    ));
    let drained = serve_until(
        ServeOptions {
            capture_protocol: ServeCaptureProtocol::FreshQualificationV2,
            listen: "127.0.0.1:0".parse()?,
            ..options
        },
        std::future::ready(()),
    )
    .await?;
    check!(eq; drained.panicked, 0);
    check!(eq; Store::capture_qualification_control_existing(&path)?.show()?, census_domain::model::CaptureQualification::V2(qualification));
    Ok(())
}

#[tokio::test]
async fn capture_v2_serve_never_initializes_absent_or_upgrades_historical_root() -> TestResult {
    let parent = tempfile::tempdir()?;
    let absent = parent.path().join("absent");
    let options = ServeOptions {
        data_dir: absent.clone(),
        capture_protocol: ServeCaptureProtocol::FreshQualificationV2,
        ..ServeOptions::default()
    };
    let error = serve_until(options, std::future::ready(()))
        .await
        .err()
        .ok_or("absent root was initialized")?
        .downcast::<BootstrapError>()?;
    let BootstrapError::StoreOpen { path, source } = error else {
        return Err("absent root did not refuse at Store open".into());
    };
    check!(eq; path, absent);
    let census_store::StoreError::Io { path, source } = *source else {
        return Err("absent root did not report missing marker".into());
    };
    check!(eq; path, absent.join("fjall").join("version"));
    check!(eq; source.kind(), std::io::ErrorKind::NotFound);
    check!(!absent.exists());
    let historical = parent.path().join("historical");
    let qualification = Store::create_capture_qualification_v1(&historical)?;
    let error = serve_until(
        ServeOptions {
            data_dir: historical.clone(),
            capture_protocol: ServeCaptureProtocol::FreshQualificationV2,
            ..ServeOptions::default()
        },
        std::future::ready(()),
    )
    .await
    .err()
    .ok_or("v2 opener admitted v1")?
    .downcast::<BootstrapError>()?;
    let BootstrapError::StoreOpen { source, .. } = error else {
        return Err("historical root did not refuse at Store open".into());
    };
    check!(matches!(
        *source,
        census_store::StoreError::Capture(census_store::CaptureError::CaptureProtocolUnsupported)
    ));
    check!(eq; Store::capture_qualification_control_existing(&historical)?.show()?, census_domain::model::CaptureQualification::V1(qualification));
    Ok(())
}
