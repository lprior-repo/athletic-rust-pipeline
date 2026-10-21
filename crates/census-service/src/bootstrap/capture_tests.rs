use super::*;
use crate::bootstrap::ServeCaptureProtocol;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn missing_capture_mode_refuses_before_store_creation_or_listener() -> TestResult {
    let parent = tempfile::tempdir()?;
    let absent = parent.path().join("absent");
    let occupied = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
    let options = ServeOptions {
        data_dir: absent.clone(),
        listen: occupied.local_addr()?,
        ..ServeOptions::default()
    };
    let error = supervise(options, std::future::ready(()))
        .await
        .err()
        .ok_or("missing acquisition authority was admitted")?;
    check!(matches!(
        error,
        BootstrapError::CaptureProtocolUnsupported { .. }
    ));
    check!(!absent.exists());
    Ok(())
}

#[tokio::test]
async fn current_capture_serve_never_initializes_absent_root() -> TestResult {
    let parent = tempfile::tempdir()?;
    let absent = parent.path().join("absent");
    let options = ServeOptions {
        data_dir: absent.clone(),
        capture_protocol: Some(ServeCaptureProtocol::FreshQualificationV3),
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
    let census_store::StoreError::Io { source, .. } = *source else {
        return Err("missing marker did not produce an I/O refusal".into());
    };
    check!(eq; source.kind(), std::io::ErrorKind::NotFound);
    check!(!absent.exists());
    Ok(())
}

#[tokio::test]
async fn current_capture_serve_preserves_identity_only_and_historical_roots() -> TestResult {
    let parent = tempfile::tempdir()?;
    let identity = parent.path().join("identity");
    let historical = parent.path().join("historical");
    Store::create_public_identity_qualification_v3(&identity)?;
    let original = Store::create_historical_capture_v1(&historical)?;
    for path in [&identity, &historical] {
        let before = Store::format(path)?;
        let options = ServeOptions {
            data_dir: path.clone(),
            capture_protocol: Some(ServeCaptureProtocol::FreshQualificationV3),
            ..ServeOptions::default()
        };
        let error = serve_until(options, std::future::ready(()))
            .await
            .err()
            .ok_or("unqualified acquisition root was admitted")?
            .downcast::<BootstrapError>()?;
        let BootstrapError::StoreOpen { source, .. } = error else {
            return Err("acquisition authority did not refuse at Store open".into());
        };
        let census_store::StoreError::Capture(refusal) = *source else {
            return Err("unqualified root did not refuse acquisition capability".into());
        };
        if path == &identity {
            check!(matches!(
                refusal,
                census_store::CaptureError::CaptureCapabilityMissing
            ));
        } else {
            check!(matches!(
                refusal,
                census_store::CaptureError::CaptureProtocolUnsupported
            ));
        }
        check!(eq; Store::format(path)?, before);
    }
    check!(eq; Store::capture_qualification_control_existing(&historical)?.show()?,
        census_domain::model::CaptureQualification::V1(original));
    check!(Store::capture_qualification_control_existing(&identity).is_err());
    Ok(())
}
