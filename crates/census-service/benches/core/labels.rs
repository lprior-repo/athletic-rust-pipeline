//! The synthetic school snapshot and label corpus the `school_index` group measures.
//!
//! Every label is *built* for the match the resolver documents, so the corpus is that contract
//! turned into data: each school contributes the spellings a timer publishes (canonical, uppercase,
//! hyphenated), its relay-squad form (`Homestead A`), and — when the name admits one — the
//! abbreviated form (`Milw. Bradley Tech`) and the fixed-column truncation (`Brookfield Cent`).
//! Decoys add labels that must stay unresolved: a single-token label (ambiguous by construction), a
//! real school published in the wrong state, a misspelling, and the empty label.
//!
//! Seed: `Lcg::seeded(SEED)` chooses the squad letter per school (`SEED` spells ASCII `LABELS_1`).
//! Everything else is a literal table, and every expectation is derived from the recipe, so the
//! corpus is byte-identical on every machine; a label that stops resolving the way it was built
//! fails [`verify`] before any rate is reported.

use anyhow::{ensure, Result};
use census_domain::model::{normalize_name, CanonicalSchool, SchoolId};
use census_domain::school_index::{SchoolIndex, SchoolMatch};
use census_domain::UsJurisdiction;
use std::collections::BTreeMap;

use super::lcg::Lcg;

/// Seed of the label stream: ASCII `LABELS_1`.
const SEED: u64 = 0x4C41_4245_4C53_5F31;
/// Relay squad letters a timer publishes after a school name.
const SQUAD_LETTERS: [char; 6] = ['A', 'B', 'C', 'D', 'E', 'F'];
/// Full word → the form a fixed-column report abbreviates it to. A literal on purpose: the corpus
/// states what it expects instead of reading the resolver's own table back out.
const ABBREVIATIONS: [(&str, &str); 5] = [
    ("milwaukee", "Milw."),
    ("university", "Univ."),
    ("catholic", "Cath."),
    ("eau claire", "EC"),
    ("wisconsin rapids", "WI Rapids"),
];
/// Characters a fixed-column report keeps of the last token of a school name.
const TRUNCATED_TAIL: usize = 4;
/// The snapshot the labels resolve against: unique normalized names inside their own state, so a
/// partial match has exactly one candidate.
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
/// Labels that must stay unresolved, and the state they are published in.
const DECOYS: [(UsJurisdiction, &str); 6] = [
    // A single token is ambiguous by construction: two schools in this state end in it.
    (UsJurisdiction::Wisconsin, "Memorial"),
    (UsJurisdiction::Wisconsin, "Central"),
    // A real school, but published in a state whose snapshot does not hold it.
    (UsJurisdiction::Minnesota, "West De Pere"),
    (UsJurisdiction::Illinois, "Milwaukee Bradley Tech"),
    // A misspelling the resolver must never guess at, and the empty label.
    (UsJurisdiction::Wisconsin, "Milwaukie Bradley Tech"),
    (UsJurisdiction::Wisconsin, ""),
];

/// The label the corpus counts under the third outcome: no school at all.
const UNRESOLVED: &str = "unresolved";
/// The outcomes that must all be present in the corpus.
const KINDS: [&str; 3] = ["exact", "abbreviation", "partial"];

/// One synthetic label and the resolution its recipe was built for.
pub struct LabelCase {
    /// The label a timer would publish.
    pub label: String,
    /// The state it is published in.
    pub state: UsJurisdiction,
    /// The school and match kind the resolver must report, or `None` when the label must stay
    /// unresolved.
    expected: Option<(SchoolId, SchoolMatch)>,
}

/// The snapshot, the label corpus, and the resolver the benches measure.
pub struct Corpus {
    schools: Vec<CanonicalSchool>,
    cases: Vec<LabelCase>,
    index: SchoolIndex,
}

impl Corpus {
    /// Build the corpus and refuse to hand it over unless every label resolves as built.
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

    /// The canonical schools the index is built from.
    pub fn schools(&self) -> &[CanonicalSchool] {
        &self.schools
    }

    /// The label corpus, in build order.
    pub fn cases(&self) -> &[LabelCase] {
        &self.cases
    }

    /// The resolver under measurement.
    pub fn index(&self) -> &SchoolIndex {
        &self.index
    }
}

/// One canonical school per entry, carrying the normalized name `normalize_name` derives from it.
fn snapshot() -> Vec<CanonicalSchool> {
    SCHOOLS
        .iter()
        .map(|(state, name)| CanonicalSchool::new(*state, *name, normalize_name(name)).0)
        .collect()
}

/// Every label the corpus holds: one school's recipes per entry, then the decoys.
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

/// The three spellings of one name that must all normalize onto its own key: the canonical name, the
/// uppercase form a Hy-Tek header carries and the hyphenated form a URL slug carries.
fn spellings(name: &str) -> [String; 3] {
    [
        name.to_string(),
        name.to_ascii_uppercase(),
        name.replace(' ', "-"),
    ]
}

/// The label a fixed-column report publishes for `name`: the first word the abbreviation table knows,
/// in its timer form (`Milwaukee Bradley Tech` → `Milw. Bradley Tech`). `None` when no word of the
/// name is one timers abbreviate.
fn abbreviated(name: &str) -> Option<String> {
    // The table holds ASCII names only, so the lowered copy indexes into the original byte for byte.
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

/// The label a report publishes after truncating the last token (`Brookfield Central` →
/// `Brookfield Cent`). `None` for a one-token name, or one whose last token the column already fits.
fn truncated(name: &str) -> Option<String> {
    let (head, last) = name.rsplit_once(' ')?;
    if last.chars().count() <= TRUNCATED_TAIL {
        return None;
    }
    let tail: String = last.chars().take(TRUNCATED_TAIL).collect();
    Some(format!("{head} {tail}"))
}

/// Record one label that must resolve to `id` as `kind`.
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

/// Every label must resolve exactly the way its recipe built it, and every decoy must stay
/// unresolved. All four outcomes have to be present, so a recipe that stopped producing a case — a
/// name no longer truncated, a spelling that no longer normalizes onto its key — fails the run
/// instead of quietly shrinking the corpus.
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
        let seen = kinds.get(kind).copied().unwrap_or(0);
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
