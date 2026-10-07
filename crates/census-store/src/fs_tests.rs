use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::fs::{create_dir_all_synced, create_dir_all_synced_with, missing_chain};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

struct Recorder {
    seen: Vec<PathBuf>,
}

impl Recorder {
    fn new() -> Self {
        Self { seen: Vec::new() }
    }

    fn run(&mut self, path: &Path) -> TestResult<u64> {
        let mut sync = |parent: &Path| -> io::Result<()> {
            self.seen.push(parent.to_path_buf());
            Ok(())
        };
        let created = create_dir_all_synced_with(path, &mut sync)?;
        Ok(created)
    }
}

#[test]
fn creates_missing_components_and_syncs_each_new_parent() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("root");
    fs::create_dir(&root)?;
    let mut recorder = Recorder::new();
    let created = recorder.run(&root.join("a").join("b").join("c"))?;
    check!(eq; created, 3);
    check!(root.join("a").join("b").join("c").is_dir());
    check!(eq; recorder.seen, vec![root.clone(), root.join("a"), root.join("a").join("b")]);
    check!(recorder.seen.iter().all(|path| path.is_dir()));
    Ok(())
}

#[test]
fn existing_directories_are_not_synced() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("root");
    create_dir_all_synced(&root)?;
    let mut recorder = Recorder::new();
    check!(eq; recorder.run(&root)?, 0);
    check!(recorder.seen.is_empty());
    create_dir_all_synced(&root.join("a"))?;
    check!(eq; recorder.run(&root.join("a"))?, 0);
    check!(recorder.seen.is_empty());
    Ok(())
}

#[test]
fn a_file_component_is_refused_without_side_effects() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("root");
    fs::create_dir(&root)?;
    let file = root.join("file");
    fs::write(&file, b"not a directory")?;
    let mut recorder = Recorder::new();
    let error = refusal(recorder.run(&root.join("file").join("child")))?;
    check!(eq; error.kind(), io::ErrorKind::NotADirectory);
    check!(recorder.seen.is_empty());
    check!(!root.join("file").join("child").exists());
    let error = refusal(recorder.run(&file))?;
    check!(eq; error.kind(), io::ErrorKind::AlreadyExists);
    check!(error.to_string().contains("file"));
    check!(recorder.seen.is_empty());
    check!(fs::read(&file)? == b"not a directory");
    Ok(())
}

fn refusal(outcome: TestResult<u64>) -> TestResult<io::Error> {
    match outcome {
        Ok(_) => Err("a file component was accepted as a directory".into()),
        Err(error) => error
            .downcast::<io::Error>()
            .map(|error| *error)
            .map_err(|_| "the refusal was not an io error".into()),
    }
}

#[cfg(unix)]
#[test]
fn a_symlinked_ancestor_is_followed() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("root");
    let real = root.join("real");
    fs::create_dir_all(&real)?;
    std::os::unix::fs::symlink(&real, root.join("link"))?;
    let mut recorder = Recorder::new();
    check!(eq; recorder.run(&root.join("link").join("deep"))?, 1);
    check!(real.join("deep").is_dir());
    check!(eq; recorder.seen, vec![root.join("link")]);
    Ok(())
}

#[test]
fn a_relative_chain_stops_before_the_empty_parent() -> TestResult {
    let mut probed: Vec<PathBuf> = Vec::new();
    let missing = missing_chain(Path::new("relscratch/x/y"), &mut |candidate: &Path| {
        probed.push(candidate.to_path_buf());
        Ok(None)
    })?;
    check!(eq; missing, vec![
        PathBuf::from("relscratch/x/y"),
        PathBuf::from("relscratch/x"),
        PathBuf::from("relscratch"),
    ]);
    check!(probed.iter().all(|path| !path.as_os_str().is_empty()));
    Ok(())
}

#[test]
fn an_existing_ancestor_stops_the_chain() -> TestResult {
    let missing = missing_chain(Path::new("/a/b/c"), &mut |candidate: &Path| {
        Ok((candidate == Path::new("/a")).then_some(true))
    })?;
    check!(eq; missing, vec![PathBuf::from("/a/b/c"), PathBuf::from("/a/b")]);
    Ok(())
}

#[test]
fn a_non_directory_ancestor_is_refused_by_the_chain() -> TestResult {
    let outcome = missing_chain(Path::new("/a/b"), &mut |candidate: &Path| {
        Ok(if candidate == Path::new("/a") {
            Some(false)
        } else {
            None
        })
    });
    let error = match outcome {
        Ok(_) => return Err("a non-directory ancestor produced a chain".into()),
        Err(error) => error,
    };
    check!(eq; error.kind(), io::ErrorKind::AlreadyExists);
    check!(error.to_string().contains("/a"));
    Ok(())
}

#[test]
fn the_public_helper_creates_and_reuses_the_tree() -> TestResult {
    let dir = tempfile::tempdir()?;
    let target = dir.path().join("x").join("y").join("z");
    create_dir_all_synced(&target)?;
    check!(target.is_dir());
    create_dir_all_synced(&target)?;
    check!(target.is_dir());
    Ok(())
}
