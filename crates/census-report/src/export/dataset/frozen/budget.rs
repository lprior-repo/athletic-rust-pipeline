use super::codec::{FrozenCodecError, FrozenCodecResult, MAX_JSON_DEPTH, MAX_JSON_STRING_BYTES};

pub(crate) fn check_json_budget(
    what: &'static str,
    bytes: &[u8],
    max_nodes: u64,
) -> FrozenCodecResult<()> {
    let mut depth: u64 = 0;
    let mut nodes: u64 = 0;
    let mut string_bytes: u64 = 0;
    let mut in_string = false;
    let mut escaped = false;
    let mut in_scalar = false;
    for byte in bytes.iter().copied() {
        if in_string {
            advance_string(what, byte, &mut string_bytes, &mut in_string, &mut escaped)?;
            continue;
        }
        match byte {
            b'"' => {
                in_string = true;
                string_bytes = 1;
                in_scalar = false;
                nodes = bump_nodes(what, nodes, max_nodes)?;
            }
            b'{' | b'[' => {
                depth = depth.checked_add(1).ok_or_else(|| overflow(what))?;
                if depth > MAX_JSON_DEPTH {
                    return Err(FrozenCodecError::BudgetExceeded {
                        what: "frozen json depth",
                        needed: depth,
                        available: MAX_JSON_DEPTH,
                    });
                }
                in_scalar = false;
                nodes = bump_nodes(what, nodes, max_nodes)?;
            }
            b'}' | b']' => {
                depth = depth.checked_sub(1).ok_or_else(|| unbalanced(what))?;
                in_scalar = false;
            }
            b',' | b':' | b' ' | b'\t' | b'\n' | b'\r' => in_scalar = false,
            _ => {
                if !in_scalar {
                    in_scalar = true;
                    nodes = bump_nodes(what, nodes, max_nodes)?;
                }
            }
        }
    }
    if in_string {
        return Err(unbalanced(what));
    }
    if depth != 0 {
        return Err(unbalanced(what));
    }
    Ok(())
}

fn advance_string(
    what: &'static str,
    byte: u8,
    string_bytes: &mut u64,
    in_string: &mut bool,
    escaped: &mut bool,
) -> FrozenCodecResult<()> {
    *string_bytes = string_bytes.checked_add(1).ok_or_else(|| overflow(what))?;
    if *string_bytes > MAX_JSON_STRING_BYTES {
        return Err(FrozenCodecError::BudgetExceeded {
            what: "frozen json string bytes",
            needed: *string_bytes,
            available: MAX_JSON_STRING_BYTES,
        });
    }
    if *escaped {
        *escaped = false;
    } else if byte == b'\\' {
        *escaped = true;
    } else if byte == b'"' {
        *in_string = false;
    }
    Ok(())
}

fn bump_nodes(what: &'static str, nodes: u64, max_nodes: u64) -> FrozenCodecResult<u64> {
    let next = nodes.checked_add(1).ok_or_else(|| overflow(what))?;
    if next > max_nodes {
        return Err(FrozenCodecError::BudgetExceeded {
            what: "frozen json nodes",
            needed: next,
            available: max_nodes,
        });
    }
    Ok(next)
}

fn overflow(what: &'static str) -> FrozenCodecError {
    FrozenCodecError::CorruptArtifact {
        what,
        detail: "a structure counter overflowed u64".to_string(),
    }
}

fn unbalanced(what: &'static str) -> FrozenCodecError {
    FrozenCodecError::CorruptArtifact {
        what,
        detail: "unterminated string or unbalanced container".to_string(),
    }
}
