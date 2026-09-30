use anyhow::{bail, ensure, Context, Result};
use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::path::Path;

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
const MAX_SOURCE_FILES: usize = 100_000;

pub(crate) fn run(root: &Path) -> Result<()> {
    let files = crate::paths::files(
        root,
        &[".git", ".jj", "target", "var", "vendor", "node_modules"],
    )?;
    let mut source = String::new();
    let mut checked = 0usize;
    let mut violations = 0usize;
    for path in files
        .iter()
        .filter(|path| path.extension() == Some(OsStr::new("rs")))
    {
        ensure!(
            checked < MAX_SOURCE_FILES,
            "Rust source file limit exceeded"
        );
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
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let size = file.metadata()?.len();
    ensure!(
        size <= MAX_SOURCE_BYTES,
        "source exceeds byte limit: {}",
        path.display()
    );
    source.clear();
    source.try_reserve(usize::try_from(size)?)?;
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
