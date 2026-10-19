use super::*;
use crate::installation::defaults as installation_defaults;
use std::net::TcpListener;

#[path = "fixture.rs"]
mod fixture;
use fixture::{configuration, equal, profile, TestResult};

#[test]
fn public_serve_refuses_enrollment_before_occupied_listener_and_browser_setup() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let profile = profile()?;
    store.register_profile(&profile)?;
    drop(store);
    let config = configuration(&profile)?;
    let mut control = Store::installation_control_existing(root.path())?;
    crate::installation::enroll(&mut control, &config)?;
    let expected = control.show()?;
    drop(control);
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
    let options = ServeOptions {
        data_dir: root.path().to_path_buf(),
        listen: listener.local_addr()?,
        capture_protocol: Some(crate::bootstrap::ServeCaptureProtocol::FreshQualificationV3),
        ..ServeOptions::default()
    };
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let expected_config_id = config.id()?;
        let result = crate::bootstrap::serve_until(options, std::future::ready(())).await;
        match result {
            Err(error) => {
                if !matches!(error.downcast_ref::<BootstrapError>(), Some(BootstrapError::EnrollmentOnlyUnsupported { config_id, .. }) if config_id == &expected_config_id) { return Err(format!("wrong bootstrap error={error:?}").into()); }
            }
            Ok(report) => return Err(format!("enrolled serve succeeded={report:?}").into()),
        }
        let control = Store::installation_control_existing(root.path())?;
        equal(control.show()?, expected)
    })
}
