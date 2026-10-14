use super::{CgroupMemoryReader, MemoryReader, NonZeroU64, Scope};
use std::path::PathBuf;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

struct Fixture {
    _directory: tempfile::TempDir,
    group: PathBuf,
    reader: MemoryReader,
}

impl Fixture {
    fn new() -> TestResult<Self> {
        let directory = tempfile::tempdir()?;
        let group = directory.path().join("group");
        std::fs::create_dir(&group)?;
        for (name, value) in [
            ("memory.max", "1073741824"),
            ("memory.swap.max", "0"),
            ("memory.current", "50"),
            ("memory.peak", "100"),
        ] {
            std::fs::write(group.join(name), value)?;
        }
        let membership = directory.path().join("membership");
        std::fs::write(&membership, "0::/group\n")?;
        let budget = NonZeroU64::new(1_073_741_824).ok_or("zero fixture budget")?;
        let cgroup = CgroupMemoryReader::with_paths(budget, membership, directory.path().into())?;
        let reader = MemoryReader {
            scope: Scope::Cgroup {
                reader: cgroup,
                peak_bytes: 100,
            },
            budget_bytes: budget.get(),
        };
        Ok(Self {
            _directory: directory,
            group,
            reader,
        })
    }
}

#[test]
fn an_observed_high_water_reset_cannot_erase_the_admitted_memory_history() -> TestResult {
    let mut fixture = Fixture::new()?;
    check!(eq; fixture.reader.read(), Some(50));
    std::fs::write(fixture.group.join("memory.peak"), "200")?;
    check!(eq; fixture.reader.read(), Some(50));
    std::fs::write(fixture.group.join("memory.peak"), "199")?;
    check!(eq; fixture.reader.read(), None);
    Ok(())
}

#[test]
fn a_tighter_limit_still_invalidates_the_original_envelope() -> TestResult {
    let mut fixture = Fixture::new()?;
    std::fs::write(fixture.group.join("memory.max"), "805306368")?;
    check!(eq; fixture.reader.read(), None);
    Ok(())
}

#[test]
fn a_peak_below_current_usage_is_not_an_acceptable_measurement() -> TestResult {
    let mut fixture = Fixture::new()?;
    std::fs::write(fixture.group.join("memory.current"), "101")?;
    check!(eq; fixture.reader.read(), None);
    Ok(())
}
