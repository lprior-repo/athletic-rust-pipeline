use super::rows;

#[test]
fn a_page_with_no_competition_rows_is_an_empty_schedule() -> Result<(), Box<dyn std::error::Error>>
{
    let empty = rows("<html><body><table></table></body></html>", 2026)?;
    check!(
        empty.is_empty(),
        "a schedule without competitions yields no rows: {empty:?}"
    );
    Ok(())
}
