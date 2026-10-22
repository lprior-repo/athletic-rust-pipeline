use super::*;
use calamine::Reader;

pub(super) fn observe(workbook: &Path, expected: &Expected<'_>) -> TestResult {
    check!(std::fs::metadata(workbook)?.len() <= 16 * 1024 * 1024);
    let mut book: calamine::Xlsx<std::io::BufReader<std::fs::File>> =
        calamine::open_workbook(workbook)?;
    check!(eq; book.sheet_names(), vec!["Athletes".to_owned()]);
    let prs = artifacts::sheet(&mut book, "Athletes")?;
    check!(eq; prs.len(),1);
    let pr = prs.first().ok_or("genuine Tessa PR workbook row")?;
    artifacts::fields(
        pr,
        &[
            ("Athlete ID", expected.public.as_str()),
            ("Name", "Tessa Mize"),
            ("Graduation Year", "2027"),
            ("Current school", "Roncalli"),
            ("Current state", "IN"),
            ("Current school as-of", "2026-10-21"),
            ("School", "Roncalli"),
            ("School State", "IN"),
        ],
    )?;
    let bests = pr.get("Observed bests").ok_or("genuine observed bests")?;
    for (label, value) in [
        ("Calculated PR", "11:23.56"),
        ("Published mark", "11:29.42"),
        ("PR date", "2026-03-11"),
        ("Performance ID", expected.performance.id.as_str()),
        ("Source Key", expected.performance.source_key.as_str()),
        ("School", "Roncalli"),
        ("State", "IN"),
    ] {
        check!(bests.contains(&format!("{label}: {value}")), "{label}");
    }
    check!(!bests.contains("; Place:"));
    let basis = expected
        .selection
        .result
        .adjustment_basis
        .ok_or("actual conversion basis")?;
    let note = expected
        .selection
        .result
        .adjustment_note
        .as_deref()
        .ok_or("actual conversion note")?;
    for (label, value) in [
        ("Conversion basis", basis.label()),
        ("Conversion note", note),
    ] {
        check!(bests.contains(&format!("{label}: {value}")));
    }
    let athletes = artifacts::sheet(&mut book, "Athletes")?;
    artifacts::fields(
        artifacts::unique(&athletes, "Athlete ID", expected.public.as_str())?,
        &[
            ("School ID", expected.school.as_str()),
            ("School", "Roncalli"),
            ("School State", "IN"),
        ],
    )?;
    let directory = workbook.parent().ok_or("published workbook directory")?;
    let rows = artifacts::csv(&directory.join("recruiting.csv"))?;
    artifacts::fields(
        artifacts::unique(&rows, "athlete_id", expected.public.as_str())?,
        &[("school", "Roncalli"), ("state", "IN")],
    )?;
    Ok(())
}
