//! The walker's own judgements: which directories are product roots, and which files of a root are
//! scanned.
//!
//! The fixtures are real directories, because the walker asks the filesystem which roots exist — a
//! table asserted against source text would pass while `roots` still walked the wrong set.

use super::*;
use crate::scan::root_files;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

/// A scratch directory, removed when the test ends, named for the process so that tests running in
/// parallel cannot share one.
struct Fixture {
    path: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("xtask-scan-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("the fixture directory is creatable");
        Self { path }
    }

    /// Create `relative` as a directory, parents included, and return it.
    fn dir(&self, relative: &str) -> PathBuf {
        let path = self.path.join(relative);
        fs::create_dir_all(&path).expect("a fixture directory is creatable");
        path
    }

    /// Create `relative` as a file, parents included, and return it.
    fn file(&self, relative: &str) -> PathBuf {
        let path = self.path.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("a fixture directory is creatable");
        }
        fs::write(&path, "pub fn stub() {}\n").expect("a fixture file is writable");
        path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// The roots as `(file name, harness)` pairs, so an assertion names what the walker took.
fn walked(roots: &[Root]) -> BTreeMap<String, bool> {
    roots
        .iter()
        .map(|root| {
            let name = root
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            (name, root.harness)
        })
        .collect()
}

/// The scanned file names of one root, relative to it.
fn scanned(root: &Root) -> Vec<String> {
    root_files("pkg", root)
        .expect("a fixture root is listable")
        .into_iter()
        .filter_map(|file| {
            file.path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .collect()
}

#[test]
fn every_root_the_walker_can_walk_is_walked() {
    let fixture = Fixture::new("roots");
    for dir in ["src", "examples", "benches", "kani", "fuzz/fuzz_targets"] {
        fixture.dir(dir);
    }
    fixture.file("build.rs");

    assert_eq!(
        walked(&roots(&fixture.path)),
        BTreeMap::from([
            ("benches".to_string(), true),
            ("build.rs".to_string(), false),
            ("examples".to_string(), true),
            ("fuzz_targets".to_string(), true),
            ("kani".to_string(), true),
            ("src".to_string(), false),
        ])
    );
}

#[test]
fn a_package_with_no_such_directory_has_no_such_root() {
    let fixture = Fixture::new("absent");
    fixture.dir("src");

    assert_eq!(
        walked(&roots(&fixture.path)),
        BTreeMap::from([("src".to_string(), false)])
    );
}

#[test]
fn an_example_target_is_harness_code_and_a_build_script_is_not() {
    let fixture = Fixture::new("judgement");
    fixture.dir("examples");
    fixture.file("build.rs");

    let roots = walked(&roots(&fixture.path));
    assert_eq!(roots.get("examples"), Some(&true));
    assert_eq!(roots.get("build.rs"), Some(&false));
}

#[test]
fn a_fuzz_directory_that_is_its_own_workspace_is_not_a_root() {
    let fixture = Fixture::new("fuzz-workspace");
    fixture.dir("fuzz/fuzz_targets");
    fixture.file("fuzz/Cargo.toml");

    assert!(walked(&roots(&fixture.path)).is_empty());
}

#[test]
fn a_build_script_is_a_root_of_one_file() {
    let fixture = Fixture::new("one-file");
    let script = fixture.file("build.rs");

    let root = Root {
        path: script,
        harness: false,
    };
    assert_eq!(scanned(&root), vec!["build.rs".to_string()]);
}

#[test]
fn files_that_name_themselves_tests_are_not_scanned() {
    let fixture = Fixture::new("test-files");
    let src = fixture.dir("src");
    fixture.file("src/mod.rs");
    fixture.file("src/store_tests.rs");
    fixture.file("src/tests/row.rs");

    let root = Root {
        path: src,
        harness: false,
    };
    assert_eq!(scanned(&root), vec!["mod.rs".to_string()]);
}
