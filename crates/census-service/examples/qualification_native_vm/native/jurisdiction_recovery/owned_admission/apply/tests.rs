use super::validate;
use anyhow::{ensure, Result};
use census_store::{Application, Receipt};

fn receipt() -> Receipt {
    Receipt {
        operation: format!("public_resource_capture_v1:declare:{}", "a".repeat(64)),
        digest: "b".repeat(64),
        at: "2026-10-21".to_owned(),
        appended: 0,
        credited: false,
    }
}

#[test]
fn normal_written_or_repeated_zero_effect_declaration_acknowledgements_pass() -> Result<()> {
    validate(&Application::Written(receipt()))?;
    validate(&Application::Repeated(receipt()))?;
    Ok(())
}

#[test]
fn other_operations_row_effects_and_credit_do_not_qualify_admission() -> Result<()> {
    let mut other = receipt();
    other.operation = "unrelated".to_owned();
    ensure!(
        validate(&Application::Written(other)).is_err(),
        "foreign operation accepted"
    );
    let mut effects = receipt();
    effects.appended = 1;
    ensure!(
        validate(&Application::Written(effects)).is_err(),
        "row effect accepted"
    );
    let mut credit = receipt();
    credit.credited = true;
    ensure!(
        validate(&Application::Repeated(credit)).is_err(),
        "credit accepted"
    );
    Ok(())
}
