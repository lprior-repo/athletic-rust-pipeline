use super::lexer::{tokens, Kind, Token};
use super::lexical::Violation;
use anyhow::{anyhow, ensure, Context, Result};
use std::path::Path;

#[path = "extraction/policy.rs"]
mod policy;

#[cfg(test)]
#[path = "extraction/refusal_tests.rs"]
mod refusal_tests;
#[cfg(test)]
#[path = "extraction/tests.rs"]
mod tests;
#[cfg(test)]
#[path = "extraction/traversal_tests.rs"]
mod traversal_tests;

const REFERENCE_REASON: &str = "unwrap-family or expect reference is forbidden";
const SUPPRESSION_REASON: &str = "panic-extraction lint suppression is forbidden";
const MAX_ATTRIBUTE_DEPTH: usize = 128;

pub(crate) fn run(root: &Path) -> Result<()> {
    let files = super::source_files(root)?;
    ensure!(
        !files.is_empty(),
        "no project-owned Rust source files found"
    );
    let mut source = String::new();
    let violations = files.iter().try_fold(0usize, |count, path| {
        super::read_source(path, &mut source)?;
        let relative = path
            .strip_prefix(root)
            .context("source outside repository")?;
        count
            .checked_add(report(relative, &source)?)
            .context("violation count overflow")
    })?;
    let templates = ["adapter", "parse", "map"].into_iter().try_fold(
        0usize,
        |count, name| -> Result<usize> {
            let (path, rendered) = match name {
                "adapter" => (
                    Path::new("<rendered adapter_module>"),
                    crate::templates::adapter_module("panic_extraction_probe"),
                ),
                "parse" => (
                    Path::new("<rendered parse_module>"),
                    crate::templates::parse_module("panic_extraction_probe"),
                ),
                "map" => (
                    Path::new("<rendered map_module>"),
                    crate::templates::map_module(),
                ),
                _ => return Err(anyhow!("unknown Rust template")),
            };
            count
                .checked_add(report(path, &rendered)?)
                .context("template violation count overflow")
        },
    )?;
    let total = violations
        .checked_add(templates)
        .context("violation count overflow")?;
    ensure!(total == 0, "panic-extraction policy: {violations} Rust files and {templates} rendered templates contain violations");
    println!(
        "panic-extraction policy: {} Rust files and 3 rendered templates checked",
        files.len()
    );
    Ok(())
}

fn report(path: &Path, source: &str) -> Result<usize> {
    let finding =
        first_violation(source).with_context(|| format!("{}: source refused", path.display()))?;
    let Some(finding) = finding else { return Ok(0) };
    let line = line_number(source, finding.offset)?;
    eprintln!("{}:{line}: {}", path.display(), finding.reason);
    Ok(1)
}

fn line_number(source: &str, offset: usize) -> Result<usize> {
    source
        .get(..offset)
        .context("invalid source position")?
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        .checked_add(1)
        .context("source line count overflow")
}

fn first_violation(source: &str) -> Result<Option<Violation>> {
    first_violation_over(tokens(source), source)
}

fn first_violation_over(
    mut stream: impl Iterator<Item = Token>,
    source: &str,
) -> Result<Option<Violation>> {
    ensure!(
        u64::try_from(source.len())? <= super::MAX_SOURCE_BYTES,
        "Rust source exceeds byte limit"
    );
    let mut offset = 0usize;
    let mut previous = [""; 3];
    let mut attribute = Attribute::default();
    let found = stream.try_fold(None, |found, token| -> Result<Option<Violation>> {
        ensure!(token.len > 0, "zero-length Rust token");
        let end = offset
            .checked_add(token.len)
            .context("Rust token position overflow")?;
        let text = source.get(offset..end).context("invalid Rust token span")?;
        let start = offset;
        offset = end;
        ensure_terminated(token.kind, source, start)?;
        if matches!(
            token.kind,
            Kind::Whitespace | Kind::LineComment | Kind::BlockComment { .. }
        ) {
            return Ok(found);
        }
        let text = if matches!(token.kind, Kind::Literal { .. }) {
            ""
        } else {
            text
        };
        let name = text.strip_prefix("r#").map_or(text, |name| name);
        let reference = matches!(token.kind, Kind::Ident | Kind::RawIdent)
            && policy::forbidden_method(name)
            && ((previous[2] == "." && previous[1] != ".") || previous[1..] == [":", ":"]);
        let suppression = attribute.mode() == Mode::Lints
            && ((name == "warnings" && previous[2] != ":")
                || (previous == ["clippy", ":", ":"]
                    && matches!(name, "unwrap_used" | "expect_used" | "restriction" | "all")));
        attribute.advance(name)?;
        previous = [previous[1], previous[2], name];
        let reason = if reference {
            Some(REFERENCE_REASON)
        } else if suppression {
            Some(SUPPRESSION_REASON)
        } else {
            None
        };
        Ok(found.or_else(|| {
            reason.map(|reason| Violation {
                offset: start,
                reason,
            })
        }))
    })?;
    ensure!(offset == source.len(), "incomplete Rust token stream");
    ensure!(attribute.depth == 0, "unterminated Rust attribute");
    Ok(found)
}

fn ensure_terminated(kind: Kind, source: &str, offset: usize) -> Result<()> {
    let label = match kind {
        Kind::Literal { terminated: false } => "literal",
        Kind::BlockComment { terminated: false } => "block comment",
        _ => return Ok(()),
    };
    Err(anyhow!(
        "line {}: unterminated Rust {label} at byte {offset}",
        line_number(source, offset)?
    ))
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Mode {
    #[default]
    Other,
    Meta,
    Condition,
    Lints,
}

#[derive(Clone, Copy)]
struct Frame {
    close: &'static str,
    mode: Mode,
    pending: Mode,
    head: bool,
}

impl Frame {
    const EMPTY: Self = Self {
        close: "",
        mode: Mode::Other,
        pending: Mode::Other,
        head: true,
    };
}

struct Attribute {
    frames: [Frame; MAX_ATTRIBUTE_DEPTH],
    depth: usize,
    prefix: bool,
}

impl Default for Attribute {
    fn default() -> Self {
        Self {
            frames: [Frame::EMPTY; MAX_ATTRIBUTE_DEPTH],
            depth: 0,
            prefix: false,
        }
    }
}

impl Attribute {
    fn mode(&self) -> Mode {
        self.depth
            .checked_sub(1)
            .and_then(|index| self.frames.get(index))
            .map_or(Mode::Other, |frame| frame.mode)
    }

    fn push(&mut self, close: &'static str, mode: Mode) -> Result<()> {
        let slot = self
            .frames
            .get_mut(self.depth)
            .context("Rust attribute nesting exceeds 128")?;
        *slot = Frame {
            close,
            mode,
            ..Frame::EMPTY
        };
        self.depth = self
            .depth
            .checked_add(1)
            .context("Rust attribute depth overflow")?;
        Ok(())
    }

    fn advance(&mut self, text: &str) -> Result<()> {
        if self.depth == 0 {
            match text {
                "#" => self.prefix = true,
                "!" if self.prefix => {}
                "[" if self.prefix => {
                    self.prefix = false;
                    self.push("]", Mode::Meta)?;
                }
                _ => self.prefix = false,
            }
            return Ok(());
        }
        let index = self
            .depth
            .checked_sub(1)
            .context("invalid Rust attribute depth")?;
        let frame = self
            .frames
            .get_mut(index)
            .context("invalid Rust attribute frame")?;
        match text {
            "(" | "[" | "{" => {
                let mode = if text == "(" {
                    frame.pending
                } else {
                    Mode::Other
                };
                frame.pending = Mode::Other;
                frame.head = false;
                let close = match text {
                    "(" => ")",
                    "[" => "]",
                    _ => "}",
                };
                self.push(close, mode)?;
            }
            ")" | "]" | "}" => {
                ensure!(text == frame.close, "mismatched Rust attribute delimiter");
                self.depth = index;
            }
            "," if frame.mode == Mode::Condition || frame.mode == Mode::Meta => {
                frame.mode = Mode::Meta;
                frame.head = true;
                frame.pending = Mode::Other;
            }
            _ if frame.mode == Mode::Meta && frame.head => {
                frame.pending = match text {
                    "allow" | "expect" => Mode::Lints,
                    "cfg_attr" => Mode::Condition,
                    _ => Mode::Other,
                };
                frame.head = false;
            }
            _ => frame.pending = Mode::Other,
        }
        Ok(())
    }
}
