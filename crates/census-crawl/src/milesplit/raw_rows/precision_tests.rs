use crate::milesplit::raw::parse_raw;
use census_domain::model::Mark;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn over_precision_time_is_retained_raw_not_rounded_to_distance_and_later_time_stays_exact(
) -> TestResult {
    let header = format!(
        "  Pl {:<20} {:<3} {:<20}{:>15}",
        "Name", "Yr", "Team", "Time"
    );
    let rejected = format!(
        "   1 {:<20} {:<3} {:<20}{:>15} 8 (1)",
        "Runner, Earlier", "JR", "Source School", "10.9410000001"
    );
    let valid = format!(
        "   2 {:<20} {:<3} {:<20}{:>15} 8 (1)",
        "Runner, Later", "JR", "Source School", "10.944"
    );
    let html = format!("<script type=\"application/ld+json\">{{\"name\":\"Exact Source Meet\",\"startDate\":\"2026-05-01\",\"sport\":\"Track\"}}</script><pre>Boys 100M\n{header}\n{rejected}\n{valid}\n</pre>");
    let parsed = parse_raw(
        &html,
        "https://www.milesplit.com/meets/498412/results/1283641/raw",
    )?;
    let event = parsed.meet.events.first().ok_or("missing source event")?;
    check!(eq; event.rows.len(), 2);
    let rejected = event
        .rows
        .iter()
        .find(|row| row.name == "Runner, Earlier")
        .ok_or("missing rejected source row")?;
    check!(eq; rejected.mark, Mark::Raw("10.9410000001".to_string()));
    let later = event
        .rows
        .iter()
        .find(|row| row.name == "Runner, Later")
        .ok_or("missing later source row")?;
    let Mark::TimeSeconds(time) = &later.mark else {
        return Err("later row lost exact time".into());
    };
    check!(eq; (time.value(), time.precision()), (10_944_000_000, 3));
    Ok(())
}
