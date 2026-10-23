use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn physical_byte_accounting_accepts_original_failed_demand_and_exact_new_limit() -> TestResult {
    let mut original = Closure::new(Table::Athletes, OBSERVATION_BUDGET)?;
    original.bytes = 4_294_967_295;
    original.push(b"a", &vec![b'x'; 1529])?;
    check!(eq; original.finish().0.encoded_bytes, 4_294_968_825);
    let limit = 16 * 1024 * 1024 * 1024;
    let mut exact = Closure::new(Table::Athletes, OBSERVATION_BUDGET)?;
    exact.bytes = limit - 2;
    exact.push(b"a", b"x")?;
    check!(eq; exact.finish().0.encoded_bytes, limit);
    let mut exceeded = Closure::new(Table::Athletes, OBSERVATION_BUDGET)?;
    exceeded.bytes = limit - 2;
    check!(
        matches!(exceeded.push(b"a", b"xx"), Err(StoreError::BudgetExceeded {
        what: "physical proof bytes", needed, available,
    }) if needed == limit + 1 && available == limit)
    );
    Ok(())
}

#[test]
fn physical_record_limit_remains_one_mib_and_counter_overflow_refuses() -> TestResult {
    let mut exact = Closure::new(Table::Athletes, OBSERVATION_BUDGET)?;
    exact.push(b"a", &vec![b'x'; 1024 * 1024])?;
    check!(eq; exact.finish().0.key_visits, 1);
    let mut exceeded = Closure::new(Table::Athletes, OBSERVATION_BUDGET)?;
    check!(matches!(
        exceeded.push(b"a", &vec![b'x'; 1024 * 1024 + 1]),
        Err(StoreError::BudgetExceeded {
            what: "physical proof record bytes",
            needed: 1_048_577,
            available: 1_048_576,
        })
    ));
    check!(matches!(
        charge(u64::MAX, 1, u64::MAX, "physical proof bytes"),
        Err(StoreError::CounterOverflow)
    ));
    Ok(())
}
