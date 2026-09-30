use super::lexical::{first_violation, Violation};
use anyhow::Result;

const CORPUS: &str = include_str!("../../tests/golden/comments_lexer_corpus.txt");
const BASELINE: &str = include_str!("../../tests/golden/comments_lexer_baseline.txt");
const SEPARATOR: &str = "-----snip-----\n";

pub(super) fn blocks() -> Vec<&'static str> {
    CORPUS.split(SEPARATOR).collect()
}

pub(super) fn render(verdict: Result<Option<Violation>>) -> String {
    match verdict {
        Ok(None) => "clean".to_string(),
        Ok(Some(Violation { offset, reason })) => format!("{reason}@{offset}"),
        Err(error) => format!("error: {error}"),
    }
}

pub(super) fn record(text: &str) -> String {
    render(first_violation(text))
}

#[test]
fn lexer_matches_the_captured_baseline() {
    let corpus = blocks();
    let rendered = corpus
        .iter()
        .map(|block| record(block))
        .collect::<Vec<String>>();
    let expected = BASELINE
        .lines()
        .map(|line| line.split_once('\t').map_or("", |(_, verdict)| verdict))
        .collect::<Vec<&str>>();
    assert_eq!(
        rendered.len(),
        expected.len(),
        "corpus and baseline differ in size"
    );
    for (index, (mine, theirs)) in rendered.iter().zip(expected.iter()).enumerate() {
        assert_eq!(mine, theirs, "block {index}: {:?}", corpus[index]);
    }
}

#[test]
fn the_corpus_stays_a_real_corpus() {
    let blocks = blocks();
    assert!(blocks.len() >= 100, "corpus shrank to {}", blocks.len());
    assert!(blocks
        .iter()
        .any(|block| record(block).starts_with("code comments are forbidden")));
    assert!(blocks
        .iter()
        .any(|block| record(block).starts_with("documentation attributes are forbidden")));
    assert!(blocks
        .iter()
        .any(|block| record(block).starts_with("error:")));
    assert!(blocks.iter().any(|block| record(block) == "clean"));
}
