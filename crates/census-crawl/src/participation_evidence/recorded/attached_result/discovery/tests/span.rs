use super::*;

#[test]
fn exact_canonical_span_accepts_zero_start_and_compares_only_original_bytes() -> TestResult {
    let span = OriginalResultSpan::parse(b"html_bytes:0:7").ok_or("canonical span")?;
    check!(span.matches(SourceRowLocator {
        ordinal: 19,
        byte_offset: 0,
        byte_length: 7
    }));
    check!(span.matches(SourceRowLocator {
        ordinal: 90,
        byte_offset: 0,
        byte_length: 7
    }));
    check!(!span.matches(SourceRowLocator {
        ordinal: 19,
        byte_offset: 1,
        byte_length: 6
    }));
    check!(!span.matches(SourceRowLocator {
        ordinal: 19,
        byte_offset: 0,
        byte_length: 6
    }));
    Ok(())
}

#[test]
fn numeric_equivalents_non_ascii_and_noncanonical_locator_spellings_refuse() -> TestResult {
    for bytes in [
        b"html_bytes:00:7".as_slice(),
        b"html_bytes:0:07",
        b"html_bytes:+0:7",
        b"html_bytes:0:+7",
        b"html_bytes:-1:7",
        b"html_bytes:1:1",
        b"html_bytes:2:1",
        b"html_bytes::7",
        b"html_bytes:0:",
        b"html_bytes: 0:7",
        b"html_bytes:0:7 ",
        b"html_bytes:0:7:8",
        b"HTML_BYTES:0:7",
        b"html_bytes:0:7\n",
        b"html_bytes:0:\xff",
        "html_bytes:０:7".as_bytes(),
        b"html_bytes:0:7e0",
    ] {
        check!(OriginalResultSpan::parse(bytes).is_none(), "{bytes:?}");
    }
    Ok(())
}

#[test]
fn locator_machine_bounds_and_overflow_are_checked_before_matching() -> TestResult {
    let last = usize::MAX.checked_sub(1).ok_or("usize maximum underflow")?;
    let span = OriginalResultSpan::parse(format!("html_bytes:{last}:{}", usize::MAX).as_bytes())
        .ok_or("maximum exact span")?;
    check!(span.matches(SourceRowLocator {
        ordinal: 1,
        byte_offset: last,
        byte_length: 1
    }));
    check!(!span.matches(SourceRowLocator {
        ordinal: 1,
        byte_offset: last,
        byte_length: 2
    }));
    let overflow = format!("html_bytes:0:{}0", usize::MAX);
    check!(OriginalResultSpan::parse(overflow.as_bytes()).is_none());
    check!(
        OriginalResultSpan::parse(format!("html_bytes:{}:2", "1".repeat(42)).as_bytes()).is_none()
    );
    Ok(())
}
