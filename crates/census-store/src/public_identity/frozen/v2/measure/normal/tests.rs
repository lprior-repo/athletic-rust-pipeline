use super::*;
type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[test]
fn physical_rows_use_an_object_local_budget_and_outer_charge_happens_once() -> TestResult {
    let mut outer = ComponentBudget::new(100);
    outer.charge(10, "raw_metadata");
    let mut local = outer.clone();
    local.charge(30, "physical_row_json");
    local.charge(40, "physical_row_json");
    outer.finish_object(local, 85);
    check!(matches!(
        outer,
        ComponentBudget::Fits { remaining_bytes: 5 }
    ));
    Ok(())
}

#[test]
fn first_normal_component_refusal_is_sticky_and_has_complete_item_size() -> TestResult {
    let mut outer = ComponentBudget::new(100);
    let mut local = outer.clone();
    local.charge(70, "physical_row_json");
    local.charge(35, "physical_row_json");
    local.charge(200, "physical_row_json");
    outer.finish_object(local, 110);
    outer.charge(300, "operation_json");
    check!(matches!(
        outer,
        ComponentBudget::Exceeded {
            component: "physical_row_json",
            complete_component_bytes: 35,
            available_bytes: 30
        }
    ));
    Ok(())
}

#[test]
fn object_framing_can_exhaust_outer_budget_after_all_rows_fit() -> TestResult {
    let mut outer = ComponentBudget::new(100);
    let mut local = outer.clone();
    local.charge(90, "physical_row_json");
    outer.finish_object(local, 101);
    check!(matches!(
        outer,
        ComponentBudget::Exceeded {
            component: "object_json",
            complete_component_bytes: 101,
            available_bytes: 100
        }
    ));
    Ok(())
}
