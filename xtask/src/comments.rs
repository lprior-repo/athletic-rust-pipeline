use anyhow::{bail, ensure, Context, Result};
use std::fs::{self, OpenOptions};
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

#[path = "comments/extraction.rs"]
pub(crate) mod extraction;
#[path = "comments/lexer.rs"]
mod lexer;
#[path = "comments/lexical.rs"]
mod lexical;
#[path = "comments/literals.rs"]
mod literals;
#[cfg(test)]
#[path = "comments/parity_tests.rs"]
mod parity_tests;
#[cfg(test)]
#[path = "comments/tests.rs"]
mod tests;

const MAX_SOURCE_BYTES: u64 = 4 * 1024 * 1024;

pub(crate) fn run(root: &Path) -> Result<()> {
    let files = source_files(root)?;
    let mut source = String::new();
    let mut checked = 0usize;
    let mut violations = 0usize;
    for path in &files {
        checked = checked
            .checked_add(1)
            .context("source file count overflow")?;
        read_source(path, &mut source)?;
        if let Some(finding) = lexical::first_violation(&source)? {
            let preceding = source
                .get(..finding.offset)
                .context("invalid source position")?;
            let line = preceding
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                .checked_add(1)
                .context("source line count overflow")?;
            let relative = path
                .strip_prefix(root)
                .context("source outside repository")?;
            eprintln!("{}:{line}: {}", relative.display(), finding.reason);
            violations = violations
                .checked_add(1)
                .context("violation count overflow")?;
        }
    }
    ensure!(checked > 0, "no project-owned Rust source files found");
    if violations > 0 {
        bail!("zero-comments policy: {violations} of {checked} Rust files contain comments");
    }
    println!("zero-comments policy: {checked} Rust files checked, no comments");
    Ok(())
}

fn read_source(path: &Path, source: &mut String) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("reading the type of {}", path.display()))?;
    ensure!(
        metadata.is_file(),
        "source is not a regular file: {}",
        path.display()
    );
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .with_context(|| format!("opening {}", path.display()))?;
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file(),
        "opened source is not a regular file: {}",
        path.display()
    );
    let size = metadata.len();
    ensure!(
        size <= MAX_SOURCE_BYTES,
        "source exceeds byte limit: {}",
        path.display()
    );
    source.clear();
    source.try_reserve(usize::try_from(MAX_SOURCE_BYTES + 1)?)?;
    file.take(MAX_SOURCE_BYTES + 1)
        .read_to_string(source)
        .with_context(|| format!("reading {}", path.display()))?;
    ensure!(
        u64::try_from(source.len())? <= MAX_SOURCE_BYTES,
        "source grew beyond byte limit: {}",
        path.display()
    );
    Ok(())
}

fn source_files(root: &Path) -> Result<Vec<std::path::PathBuf>> {
    let files = crate::paths::rust_files_excluding(
        root,
        &[".git", ".jj", "target", "var", "vendor", "node_modules"],
    )?;
    ensure!(
        !files.is_empty(),
        "no project-owned Rust source files found"
    );
    Ok(files)
}
