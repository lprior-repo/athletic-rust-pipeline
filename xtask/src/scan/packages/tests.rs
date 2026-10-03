use super::*;
use crate::scan::root_files;
use anyhow::Result;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

struct Fixture {
    directory: tempfile::TempDir,
}

impl Fixture {
    fn new(name: &str) -> Result<Self> {
        let directory = tempfile::Builder::new()
            .prefix(&format!("xtask-scan-{name}-"))
            .tempdir()?;
        Ok(Self { directory })
    }

    fn path(&self) -> &std::path::Path {
        self.directory.path()
    }

    fn dir(&self, relative: &str) -> Result<PathBuf> {
        let path = self.path().join(relative);
        fs::create_dir_all(&path)?;
        Ok(path)
    }

    fn file(&self, relative: &str) -> Result<PathBuf> {
        let path = self.path().join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, "pub fn stub() {}\n")?;
        Ok(path)
    }

    fn close(self) -> Result<()> {
        self.directory.close()?;
        Ok(())
    }
}

fn walked(roots: &[Root]) -> BTreeMap<String, bool> {
    roots
        .iter()
        .map(|root| {
            let name = root.path.file_name().map_or(Default::default(), core::convert::identity)
            .to_string_lossy()
            .into_owned();
            (name, root.harness)
        })
        .collect()
}

fn scanned(root: &Root) -> Result<Vec<String>> {
    Ok(root_files("pkg", root)?
        .into_iter()
        .filter_map(|file| {
            file.path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .collect())
}

#[test]
fn every_root_the_walker_can_walk_is_walked() -> TestResult {
    let fixture = Fixture::new("roots")?;
    for dir in ["src", "examples", "benches", "kani", "fuzz/fuzz_targets"] {
        fixture.dir(dir)?;
    }
    fixture.file("build.rs")?;

    check!(eq;
        walked(&roots(fixture.path())),
        BTreeMap::from([
            ("benches".to_string(), true),
            ("build.rs".to_string(), false),
            ("examples".to_string(), true),
            ("fuzz_targets".to_string(), true),
            ("kani".to_string(), true),
            ("src".to_string(), false),
        ])
    );
    fixture.close()?;
    Ok(())
}

#[test]
fn a_package_with_no_such_directory_has_no_such_root() -> TestResult {
    let fixture = Fixture::new("absent")?;
    fixture.dir("src")?;

    check!(eq;
        walked(&roots(fixture.path())),
        BTreeMap::from([("src".to_string(), false)])
    );
    fixture.close()?;
    Ok(())
}

#[test]
fn an_example_target_is_harness_code_and_a_build_script_is_not() -> TestResult {
    let fixture = Fixture::new("judgement")?;
    fixture.dir("examples")?;
    fixture.file("build.rs")?;

    let roots = walked(&roots(fixture.path()));
    check!(eq; roots.get("examples"), Some(&true));
    check!(eq; roots.get("build.rs"), Some(&false));
    fixture.close()?;
    Ok(())
}

#[test]
fn a_fuzz_directory_that_is_its_own_workspace_is_not_a_root() -> TestResult {
    let fixture = Fixture::new("fuzz-workspace")?;
    fixture.dir("fuzz/fuzz_targets")?;
    fixture.file("fuzz/Cargo.toml")?;

    check!(walked(&roots(fixture.path())).is_empty());
    fixture.close()?;
    Ok(())
}

#[test]
fn a_build_script_is_a_root_of_one_file() -> TestResult {
    let fixture = Fixture::new("one-file")?;
    let script = fixture.file("build.rs")?;

    let root = Root {
        path: script,
        harness: false,
    };
    check!(eq; scanned(&root)?, vec!["build.rs".to_string()]);
    fixture.close()?;
    Ok(())
}

#[test]
fn files_that_name_themselves_tests_are_not_scanned() -> TestResult {
    let fixture = Fixture::new("test-files")?;
    let src = fixture.dir("src")?;
    fixture.file("src/mod.rs")?;
    fixture.file("src/store_tests.rs")?;
    fixture.file("src/tests/row.rs")?;

    let root = Root {
        path: src,
        harness: false,
    };
    check!(eq; scanned(&root)?, vec!["mod.rs".to_string()]);
    fixture.close()?;
    Ok(())
}
