use anyhow::Result;

mod checks;
mod docs;
mod registry;
mod tree;

pub(crate) struct Check {
    number: usize,
    name: &'static str,
    detail: String,
    failures: Vec<String>,
    deviation: Option<String>,
}

impl Check {
    pub(crate) fn holds(number: usize, name: &'static str, detail: String) -> Self {
        Self {
            number,
            name,
            detail,
            failures: Vec::new(),
            deviation: None,
        }
    }

    pub(crate) fn violated(
        number: usize,
        name: &'static str,
        detail: String,
        failures: Vec<String>,
    ) -> Self {
        Self {
            number,
            name,
            detail,
            failures,
            deviation: None,
        }
    }

    pub(crate) fn deviates(
        number: usize,
        name: &'static str,
        detail: String,
        deviation: String,
    ) -> Self {
        Self {
            number,
            name,
            detail,
            failures: Vec::new(),
            deviation: Some(deviation),
        }
    }
}

pub(crate) fn run() -> Result<()> {
    let checks = checks::all();
    println!("contract: {} architectural checks", checks.len());
    let mut violated = 0usize;
    let mut deviated = 0usize;
    for check in &checks {
        if !check.failures.is_empty() {
            violated = violated.saturating_add(1);
            println!(
                "  check {} {}: FAIL ({})",
                check.number, check.name, check.detail
            );
            for failure in &check.failures {
                println!("    {failure}");
            }
            continue;
        }
        if let Some(deviation) = &check.deviation {
            deviated = deviated.saturating_add(1);
            println!(
                "  check {} {}: KNOWN DEVIATION ({})",
                check.number, check.name, check.detail
            );
            println!("    {deviation}");
            continue;
        }
        println!(
            "  check {} {}: PASS ({})",
            check.number, check.name, check.detail
        );
    }
    if violated == 0 {
        println!("contract: PASS ({deviated} known deviation(s))");
        return Ok(());
    }
    anyhow::bail!(
        "{violated} of {} architectural checks are violated, {deviated} known deviation(s)",
        checks.len()
    )
}
