use super::*;

#[test]
fn one_source_result_id_in_distinct_implement_contexts_cannot_overwrite_numeric_performances(
) -> TestResult {
    let (meet, mut results) = super::specifications::blocks(&[
        ("Shot Put (4kg)", "SP", "40-0"),
        ("Shot Put (3kg)", "SP", "45-0"),
    ])?;
    results.blocks.iter_mut().try_for_each(|block| {
        let row = block.results.first_mut().ok_or("fixture result")?;
        row.result_id = 17;
        Ok::<_, Box<dyn std::error::Error>>(())
    })?;
    let walk = absorb(&meet, &results, None)?;
    let contexts = walk
        .accumulated
        .performances
        .values()
        .map(|performance| {
            let event = walk
                .accumulated
                .events
                .get(performance.event.as_str())
                .ok_or("event reference")?;
            let mass = event
                .specification
                .implement
                .ok_or("published implement")?
                .micrograms();
            Ok((mass, performance))
        })
        .collect::<TestResult<BTreeMap<_, _>>>()?;
    let four = contexts.get(&4_000_000_000).ok_or("4kg source result")?;
    let three = contexts.get(&3_000_000_000).ok_or("3kg source result")?;
    check!(eq; four.athlete, three.athlete);
    check!(eq; four.source_key, three.source_key);
    check!(four.id != three.id && four.event != three.event);
    check!(matches!(&four.mark, Mark::FieldImperial { feet_mark, .. } if feet_mark == "40-0"));
    check!(matches!(&three.mark, Mark::FieldImperial { feet_mark, .. } if feet_mark == "45-0"));
    Ok(())
}
