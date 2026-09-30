use super::super::claims::MAX_SPAN;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

const BASELINE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/golden/coachverify_spans_baseline.txt"
));
const WHITESPACE_ONLY: &str = "../census-crawl/tests/fixtures/plain_names/nsaa_directory_form.html";
const INLINE: [&str; 12] = [
    "",
    "no markup at all",
    "line one\n\nline two",
    "<h1>Mosinee High School WI</h1><table><tr><td>Dana Reid</td><td>Head XC Coach</td><td>dana@example.org</td></tr></table>",
    "<tr><td>Dana Reid Head XC Coach</td></tr><tr><td>Other Person Head XC Coach dana@example.org</td></tr>",
    "<h1>Other High School WI</h1><table><tr><td>Dana Reid Head XC Coach dana@example.org</td></tr></table>",
    "<article class=\'coach-card\'><h2>Dana Reid</h2><p>Head XC Coach dana@example.org</p></article>",
    "<div class=\'staff-card\'><span>Dana</span><span>Reid</span><span>Head XC Coach</span></div>",
    "<div><p>First paragraph</p><p>Second paragraph dana@example.org</p></div>",
    "<ul><li>Dana Reid Head XC Coach</li><li>Other Person dana@example.org</li></ul>",
    "<table><tr><td>Dana Reid</td><tr><td>Head XC Coach dana@example.org</td></table>",
    "<html><head><title>Mosinee High School WI</title><script>var x = \"<tr>fake</tr>\";</script></head><body><p>Dana Reid Head XC Coach dana@example.org</p></body></html>",
];

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "html") {
            out.push(path);
        }
    }
}

fn corpus() -> Vec<(String, String)> {
    let mut files = Vec::new();
    for root in [
        "../census-crawl/tests/fixtures",
        "../census-crawl/src/ciac/tests/fixtures",
        "../census-crawl/src/riil",
    ] {
        collect(Path::new(root), &mut files);
    }
    files.sort();
    let mut corpus: Vec<(String, String)> = files
        .into_iter()
        .filter_map(|path| {
            std::fs::read_to_string(&path)
                .ok()
                .map(|text| (path.display().to_string(), text))
        })
        .collect();
    corpus.extend(
        INLINE
            .into_iter()
            .enumerate()
            .map(|(index, text)| (format!("inline-{index}"), text.to_string())),
    );
    corpus
}

fn blocks(source: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for line in source.split_inclusive('\n') {
        if let Some(label) = line.strip_prefix("== ") {
            out.push((label.trim_end().to_string(), String::new()));
        }
        if let Some((_, text)) = out.last_mut() {
            text.push_str(line);
        }
    }
    out
}

fn rendered(label: &str, text: &str) -> String {
    let (spans, heading) = super::spans(text);
    let mut out = String::new();
    let _ = writeln!(out, "== {label}");
    let _ = writeln!(out, "H\t{heading:?}");
    for span in spans {
        let _ = writeln!(out, "S\t{span:?}");
    }
    out
}

fn collapsed(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut out = String::new();
    let mut gap = false;
    let mut index = 0;
    while index < bytes.len() {
        let ch = bytes[index] as char;
        if ch == '\\'
            && index + 1 < bytes.len()
            && matches!(bytes[index + 1] as char, 'n' | 't' | 'r')
        {
            gap = true;
            index += 2;
            continue;
        }
        if ch.is_ascii_whitespace() {
            gap = true;
            index += 1;
            continue;
        }
        if gap && !out.is_empty() {
            out.push(' ');
        }
        gap = false;
        out.push(ch);
        index += 1;
    }
    out
}

fn decoded_len(line: &str) -> usize {
    let bytes = line.as_bytes();
    let mut length = 0;
    let mut index = 0;
    while index < bytes.len() {
        let ch = bytes[index] as char;
        if ch == '\\' && index + 1 < bytes.len() {
            index += 2;
        } else {
            index += 1;
        }
        length += 1;
    }
    length
}

#[test]
fn spans_match_the_captured_scraper_baseline() {
    let expected = blocks(BASELINE);
    let actual: Vec<(String, String)> = corpus()
        .into_iter()
        .map(|(label, text)| {
            let block = rendered(&label, &text);
            (label, block)
        })
        .collect();
    assert_eq!(expected.len(), actual.len());
    let mut differing = Vec::new();
    for ((expected_label, expected_block), (label, block)) in expected.iter().zip(&actual) {
        assert_eq!(expected_label, label);
        if expected_block != block {
            differing.push(label.clone());
        }
    }
    assert_eq!(differing, vec![WHITESPACE_ONLY.to_string()]);

    let expected_block = expected
        .iter()
        .find(|(label, _)| label == WHITESPACE_ONLY)
        .map(|(_, block)| block)
        .expect("captured baseline block");
    let actual_block = actual
        .iter()
        .find(|(label, _)| label == WHITESPACE_ONLY)
        .map(|(_, block)| block)
        .expect("current block");
    let expected_spans: Vec<&str> = expected_block
        .lines()
        .skip(2)
        .map(|line| line.trim_start_matches("S\t"))
        .collect();
    let actual_spans: Vec<&str> = actual_block
        .lines()
        .skip(2)
        .map(|line| line.trim_start_matches("S\t"))
        .collect();
    assert_eq!(expected_spans.len(), actual_spans.len());
    assert_eq!(
        expected_block.lines().nth(1),
        actual_block.lines().nth(1),
        "the differing fixture must keep its heading"
    );
    let pairs: Vec<(&&str, &&str)> = expected_spans
        .iter()
        .zip(&actual_spans)
        .filter(|(expected_span, span)| expected_span != span)
        .collect();
    assert_eq!(pairs.len(), 1);
    for (expected_span, span) in pairs {
        assert_eq!(collapsed(expected_span), collapsed(span));
        assert!(decoded_len(expected_span) > MAX_SPAN);
        assert!(decoded_len(span) > MAX_SPAN);
    }
}
