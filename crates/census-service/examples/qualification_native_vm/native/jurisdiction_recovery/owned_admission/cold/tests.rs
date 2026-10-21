use super::receipt;
use anyhow::{ensure, Result};
use census_store::{Application, Receipt};

fn actual() -> Receipt {
    Receipt {
        operation: format!("public_resource_capture_v1:declare:{}", "a".repeat(64)),
        digest: "b".repeat(64),
        at: "2026-10-21T20:00:00Z".to_owned(),
        appended: 0,
        credited: false,
    }
}

#[test]
fn cold_receipt_requires_exact_scope_operation_digest_time_and_effects() -> Result<()> {
    let retained = actual();
    receipt(
        &Application::Written(retained.clone()),
        &retained,
        &retained.operation,
    )?;
    receipt(
        &Application::Repeated(retained.clone()),
        &retained,
        &retained.operation,
    )?;
    for changed in [
        Receipt {
            operation: format!("public_resource_capture_v1:declare:{}", "c".repeat(64)),
            ..actual()
        },
        Receipt {
            digest: "d".repeat(64),
            ..actual()
        },
        Receipt {
            at: "2026-10-22T20:00:00Z".to_owned(),
            ..actual()
        },
        Receipt {
            appended: 1,
            ..actual()
        },
        Receipt {
            credited: true,
            ..actual()
        },
    ] {
        ensure!(
            receipt(
                &Application::Written(changed),
                &retained,
                &retained.operation
            )
            .is_err(),
            "foreign cold receipt qualified original admission"
        );
    }
    ensure!(
        receipt(
            &Application::Written(retained.clone()),
            &retained,
            "different original scope"
        )
        .is_err(),
        "foreign expected operation qualified admission"
    );
    Ok(())
}
