use anyhow::{ensure, Result};
use census_domain::model::{normalize_name, CanonicalSchool, SchoolId};
use census_domain::school_index::{SchoolIndex, SchoolMatch};
use census_domain::UsJurisdiction;
use std::collections::BTreeMap;

use super::lcg::Lcg;

const SEED: u64 = 0x4C41_4245_4C53_5F31;
const SQUAD_LETTERS: [char; 6] = ['A', 'B', 'C', 'D', 'E', 'F'];
const ABBREVIATIONS: [(&str, &str); 5] = [
    ("milwaukee", "Milw."),
    ("university", "Univ."),
    ("catholic", "Cath."),
    ("eau claire", "EC"),
    ("wisconsin rapids", "WI Rapids"),
];
const TRUNCATED_TAIL: usize = 4;
const SCHOOLS: [(UsJurisdiction, &str); 16] = [
    (UsJurisdiction::Wisconsin, "Milwaukee Bradley Tech"),
    (UsJurisdiction::Wisconsin, "Brookfield Central"),
    (UsJurisdiction::Wisconsin, "Madison La Follette"),
    (UsJurisdiction::Wisconsin, "Wisconsin Lutheran"),
    (UsJurisdiction::Wisconsin, "Eau Claire Memorial"),
    (UsJurisdiction::Wisconsin, "Homestead"),
    (UsJurisdiction::Wisconsin, "West De Pere"),
    (UsJurisdiction::Wisconsin, "Franklin"),
    (UsJurisdiction::Wisconsin, "Wisconsin Rapids Lincoln"),
    (UsJurisdiction::Wisconsin, "Catholic Memorial"),
    (UsJurisdiction::Wisconsin, "University School of Milwaukee"),
    (UsJurisdiction::Minnesota, "Wayzata"),
    (UsJurisdiction::Minnesota, "Aitkin"),
    (UsJurisdiction::Minnesota, "Foley"),
    (UsJurisdiction::Illinois, "Lincoln Way Central"),
    (UsJurisdiction::Illinois, "Adlai Stevenson"),
];
const DECOYS: [(UsJurisdiction, &str); 6] = [
    (UsJurisdiction::Wisconsin, "Memorial"),
    (UsJurisdiction::Wisconsin, "Central"),
    (UsJurisdiction::Minnesota, "West De Pere"),
    (UsJurisdiction::Illinois, "Milwaukee Bradley Tech"),
    (UsJurisdiction::Wisconsin, "Milwaukie Bradley Tech"),
    (UsJurisdiction::Wisconsin, ""),
];

const UNRESOLVED: &str = "unresolved";
const KINDS: [&str; 3] = ["exact", "abbreviation", "partial"];

pub struct LabelCase {
    pub label: String,
    pub state: UsJurisdiction,
    expected: Option<(SchoolId, SchoolMatch)>,
}

pub struct Corpus {
    schools: Vec<CanonicalSchool>,
    cases: Vec<LabelCase>,
    index: SchoolIndex,
}

impl Corpus {
    pub fn build() -> Result<Self> {
        let schools = snapshot();
        let index = SchoolIndex::from_schools(&schools);
        let cases = labels();
        verify(&index, &cases)?;
        Ok(Self {
            schools,
            cases,
            index,
        })
    }

    pub fn schools(&self) -> &[CanonicalSchool] {
        &self.schools
    }

    pub fn cases(&self) -> &[LabelCase] {
        &self.cases
    }

    pub fn index(&self) -> &SchoolIndex {
        &self.index
    }
}

fn snapshot() -> Vec<CanonicalSchool> {
    SCHOOLS
        .iter()
        .map(|(state, name)| CanonicalSchool::new(*state, *name, normalize_name(name)).0)
        .collect()
}

fn labels() -> Vec<LabelCase> {
    let mut lcg = Lcg::seeded(SEED);
    let mut cases = Vec::new();
    for (state, name) in SCHOOLS {
        let (_, id) = CanonicalSchool::new(state, name, normalize_name(name));
        for label in spellings(name) {
            resolved(&mut cases, state, label, &id, SchoolMatch::Exact);
        }
        let letter = lcg.pick(&SQUAD_LETTERS, 'A');
        resolved(
            &mut cases,
            state,
            format!("{name} {letter}"),
            &id,
            SchoolMatch::Exact,
        );
        if let Some(label) = abbreviated(name) {
            resolved(&mut cases, state, label, &id, SchoolMatch::Abbreviation);
        }
        if let Some(label) = truncated(name) {
            resolved(&mut cases, state, label, &id, SchoolMatch::Partial);
        }
    }
    for (state, label) in DECOYS {
        cases.push(LabelCase {
            label: label.to_string(),
            state,
            expected: None,
        });
    }
    cases
}

fn spellings(name: &str) -> [String; 3] {
    [
        name.to_string(),
        name.to_ascii_uppercase(),
        name.replace(' ', "-"),
    ]
}

fn abbreviated(name: &str) -> Option<String> {
    let lowered = name.to_ascii_lowercase();
    for (word, timer_form) in ABBREVIATIONS {
        let Some(at) = lowered.find(word) else {
            continue;
        };
        let head = name.get(..at)?;
        let tail = name.get(at.checked_add(word.len())?..)?;
        return Some(format!("{head}{timer_form}{tail}"));
    }
    None
}

fn truncated(name: &str) -> Option<String> {
    let (head, last) = name.rsplit_once(' ')?;
    if last.chars().count() <= TRUNCATED_TAIL {
        return None;
    }
    let tail: String = last.chars().take(TRUNCATED_TAIL).collect();
    Some(format!("{head} {tail}"))
}

fn resolved(
    cases: &mut Vec<LabelCase>,
    state: UsJurisdiction,
    label: String,
    id: &SchoolId,
    kind: SchoolMatch,
) {
    cases.push(LabelCase {
        label,
        state,
        expected: Some((id.clone(), kind)),
    });
}

fn verify(index: &SchoolIndex, cases: &[LabelCase]) -> Result<()> {
    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    for case in cases {
        let observed = index.resolve(case.state, &case.label);
        ensure!(
            observed == case.expected,
            "label {:?} in {} resolved to {:?}; the corpus built it as {:?}",
            case.label,
            case.state,
            observed,
            case.expected
        );
        let kind = case
            .expected
            .as_ref()
            .map_or(UNRESOLVED, |(_, kind)| kind.as_str());
        let seen = kinds.entry(kind).or_default();
        *seen = seen.saturating_add(1);
    }
    for kind in KINDS.into_iter().chain([UNRESOLVED]) {
        let seen = kinds.get(kind).copied().map_or(0, |value| value);
        ensure!(seen > 0, "the label corpus carries no {kind} case");
    }
    ensure!(
        index.school_count() == SCHOOLS.len(),
        "the snapshot lost schools: {} of {} indexed",
        index.school_count(),
        SCHOOLS.len()
    );
    Ok(())
}
