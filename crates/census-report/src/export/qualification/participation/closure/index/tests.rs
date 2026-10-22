use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn exact_id_hits_keep_all_physical_duplicates_and_related_rows_keep_global_order() -> TestResult {
    let physical_rows = [
        (2, "first"),
        (1, "other"),
        (2, "second"),
        (3, "foreign"),
        (1, "last"),
    ];
    let index = physical(&physical_rows, |row| &row.0);
    check!(eq; hits(&index, &2), vec![&(2, "first"), &(2, "second")]);
    let ids = [2, 1, 2];
    check!(eq;
        related(&index, ids.iter()),
        vec![&(2, "first"), &(1, "other"), &(2, "second"), &(1, "last")]
    );
    check!(hits(&index, &9).is_empty());
    check!(related(&index, [9].iter()).is_empty());
    Ok(())
}
