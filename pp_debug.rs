    // DEBUG: print CO2027 performance marks
    let athletes = store.scan::<CanonicalAthlete>(census_store::Table::Athletes)?;
    let perfs = store.scan::<CanonicalPerformance>(census_store::Table::Performances)?;
    for p in &perfs {
        if let Some(a) = athletes.iter().find(|a| a.id == p.athlete) {
            if a.grad_year.get() == 2027 {
                eprintln!("DEBUG CO2027 perf: athlete={} event={} mark={:?}", a.id, p.event, p.mark);
            }
        }
    }
