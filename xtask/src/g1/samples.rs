use indexmap::IndexMap;

use crate::digests::digest_head;
use crate::pyrepr::py_repr_tuple;
use crate::rows::RowAnalysis;

pub(crate) type Samples = IndexMap<String, Vec<SampleEntry>>;

pub(crate) enum SampleEntry {
    Noncanonical(String, String),
    CountGtRows(String, Option<String>, i64, usize, String),
}

pub(crate) fn push_noncanonical(
    class_samples: &mut Samples,
    digest: &str,
    row_analyses: &[RowAnalysis],
    limit: usize,
) {
    for row_a in row_analyses {
        for href in &row_a.noncanonical {
            let v = class_samples.entry("noncanonical".to_string()).or_default();
            if v.len() < limit {
                v.push(SampleEntry::Noncanonical(digest.to_string(), href.clone()));
            }
        }
    }
}

fn pager_head(pager: &str) -> String {
    if pager.len() > 200 {
        match pager.split_at_checked(200) {
            Some((head, _)) => head.to_string(),
            None => pager.to_string(),
        }
    } else {
        pager.to_string()
    }
}

pub(crate) fn push_count_gt_rows(
    class_samples: &mut Samples,
    digest: &str,
    sport: Option<&str>,
    count: i64,
    rows: usize,
    pager: &str,
    limit: usize,
) {
    let samples = class_samples
        .entry("count_gt_rows".to_string())
        .or_default();
    if samples.len() < limit {
        samples.push(SampleEntry::CountGtRows(
            digest.to_string(),
            sport.map(|s| s.to_string()),
            count,
            rows,
            pager_head(pager),
        ));
    }
}

fn print_entry(name: &str, value: &SampleEntry) {
    match value {
        SampleEntry::Noncanonical(digest, href) => {
            let d = digest_head(digest);
            let elements: Vec<_> = vec![(d, false), (href.as_str(), false)];
            println!("  [{}] {}", name, py_repr_tuple(&elements));
        }
        SampleEntry::CountGtRows(digest, sport, count, rows, pager) => {
            let d = digest_head(digest);
            let sport_str = sport.as_deref().map_or("None", core::convert::identity);
            let e1 = d;
            let e2 = sport_str;
            let e3 = count.to_string();
            let e4 = rows.to_string();
            let e5 = pager.as_str();
            let elements: Vec<_> = vec![
                (e1, false),
                (e2, sport.is_none()),
                (e3.as_str(), true),
                (e4.as_str(), true),
                (e5, false),
            ];
            println!("  [{}] {}", name, py_repr_tuple(&elements));
        }
    }
}

pub(crate) fn print(class_samples: &Samples) {
    println!("samples:");
    for (name, values) in class_samples {
        for value in values {
            print_entry(name, value);
        }
    }
}
