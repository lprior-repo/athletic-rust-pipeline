//! Append-only entity store plus streaming consolidation.
//!
//! Collection is interrupted constantly (politeness delays, network, operator), so every adapter
//! appends to per-table JSONL logs. Consolidation then streams those logs into a deduplicated
//! snapshot, merging evidence and source identities rather than overwriting them. Nothing is ever
//! rewritten in place, so a crash costs at most the current append.

use crate::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam,
};
use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Table {
    Schools,
    Teams,
    Coaches,
    Athletes,
    Meets,
    Events,
    Performances,
}

impl Table {
    pub fn file(self) -> &'static str {
        match self {
            Table::Schools => "schools",
            Table::Teams => "teams",
            Table::Coaches => "coaches",
            Table::Athletes => "athletes",
            Table::Meets => "meets",
            Table::Events => "events",
            Table::Performances => "performances",
        }
    }

    pub const ALL: [Table; 7] = [
        Table::Schools,
        Table::Teams,
        Table::Coaches,
        Table::Athletes,
        Table::Meets,
        Table::Events,
        Table::Performances,
    ];
}

/// An entity that knows its own canonical id and how to absorb a duplicate observation.
pub trait Entity: Serialize + DeserializeOwned + Clone {
    fn entity_id(&self) -> &str;
    fn merge(&mut self, other: Self);
}

pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        for sub in ["http", "entities", "journal", "out"] {
            std::fs::create_dir_all(root.join(sub))
                .with_context(|| format!("creating {}", root.join(sub).display()))?;
        }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn http_cache_dir(&self) -> PathBuf {
        self.root.join("http")
    }

    pub fn out_dir(&self) -> PathBuf {
        self.root.join("out")
    }

    pub fn table_path(&self, table: Table) -> PathBuf {
        self.root
            .join("entities")
            .join(format!("{}.jsonl", table.file()))
    }

    /// Append observations to a table log.
    pub fn append_many<T: Serialize>(&self, table: Table, records: &[T]) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }
        let path = self.table_path(table);
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("opening {}", path.display()))?;
        let mut writer = BufWriter::new(&mut file);
        for record in records {
            serde_json::to_writer(&mut writer, record)?;
            writer.write_all(b"\n")?;
        }
        writer.flush()?;
        Ok(())
    }

    pub fn append<T: Serialize>(&self, table: Table, record: &T) -> Result<()> {
        self.append_many(table, std::slice::from_ref(record))
    }

    /// Stream the append log, merge duplicates, and write a deduplicated snapshot.
    ///
    /// Returns the number of distinct entities written.
    pub fn consolidate<T: Entity>(&self, table: Table, out_path: &Path) -> Result<usize> {
        let path = self.table_path(table);
        let mut merged: HashMap<String, T> = HashMap::new();
        if path.exists() {
            let file = std::fs::File::open(&path)?;
            for (line_no, line) in BufReader::new(file).lines().enumerate() {
                let line = line?;
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let record: T = match serde_json::from_str(trimmed) {
                    Ok(record) => record,
                    Err(error) => {
                        anyhow::bail!("{} line {}: {error}", path.display(), line_no + 1);
                    }
                };
                let id = record.entity_id().to_string();
                match merged.get_mut(&id) {
                    Some(existing) => existing.merge(record),
                    None => {
                        merged.insert(id, record);
                    }
                }
            }
        }
        let mut ids: Vec<&String> = merged.keys().collect();
        ids.sort();
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut writer = BufWriter::new(std::fs::File::create(out_path)?);
        for id in &ids {
            let record = merged.get(*id).expect("id from merged map");
            serde_json::to_writer(&mut writer, record)?;
            writer.write_all(b"\n")?;
        }
        writer.flush()?;
        Ok(ids.len())
    }

    // -- journal ---------------------------------------------------------------------------------

    fn journal_path(&self, phase: &str) -> PathBuf {
        self.root.join("journal").join(format!("{phase}.jsonl"))
    }

    /// Record that a unit of work completed. Doubles as the resume ledger.
    pub fn journal_done<T: Serialize>(&self, phase: &str, key: &str, payload: &T) -> Result<()> {
        let path = self.journal_path(phase);
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        let entry = serde_json::json!({
            "key": key,
            "at": crate::net::now_iso8601(),
            "payload": payload,
        });
        serde_json::to_writer(&mut file, &entry)?;
        file.write_all(b"\n")?;
        Ok(())
    }

    /// Keys already processed for a phase — the resume set.
    pub fn journal_keys(&self, phase: &str) -> Result<HashSet<String>> {
        let path = self.journal_path(phase);
        let mut keys = HashSet::new();
        if !path.exists() {
            return Ok(keys);
        }
        for line in BufReader::new(std::fs::File::open(&path)?).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(key) = value.get("key").and_then(|v| v.as_str()) {
                    keys.insert(key.to_string());
                }
            }
        }
        Ok(keys)
    }

    /// All journal payloads for a phase (used to rebuild adapter reports).
    pub fn journal_payloads(&self, phase: &str) -> Result<Vec<serde_json::Value>> {
        let path = self.journal_path(phase);
        let mut out = Vec::new();
        if !path.exists() {
            return Ok(out);
        }
        for line in BufReader::new(std::fs::File::open(&path)?).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(payload) = value.get("payload") {
                    out.push(payload.clone());
                }
            }
        }
        Ok(out)
    }
}

fn union_vec<T: PartialEq + Clone>(left: &mut Vec<T>, right: &[T]) {
    for item in right {
        if !left.contains(item) {
            left.push(item.clone());
        }
    }
}

impl Entity for CanonicalSchool {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        if self.city.is_none() {
            self.city = other.city;
        }
        if self.association.is_none() {
            self.association = other.association;
        }
        if self.classification.is_none() {
            self.classification = other.classification;
        }
        if self.enrollment.is_none() {
            self.enrollment = other.enrollment;
        }
        if self.school_website.is_none() {
            self.school_website = other.school_website;
        }
        if self.athletics_website.is_none() {
            self.athletics_website = other.athletics_website;
        }
        self.co_op |= other.co_op;
        if other.name.len() > self.name.len() && self.name.starts_with(&other.name) {
            // keep the longer, more specific name
            self.name = other.name;
        }
        union_vec(&mut self.aliases, &other.aliases);
        union_vec(&mut self.source_identities, &other.source_identities);
        union_vec(&mut self.evidence, &other.evidence);
    }
}

impl Entity for CanonicalTeam {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        if self.level.is_none() {
            self.level = other.level;
        }
        union_vec(&mut self.source_identities, &other.source_identities);
        union_vec(&mut self.evidence, &other.evidence);
    }
}

impl Entity for CanonicalCoach {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        if self.professional_email.is_none() {
            self.professional_email = other.professional_email;
        }
        if self.phone.is_none() {
            self.phone = other.phone;
        }
        union_vec(&mut self.source_identities, &other.source_identities);
        union_vec(&mut self.evidence, &other.evidence);
    }
}

impl Entity for CanonicalAthlete {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        union_vec(&mut self.known_names, &other.known_names);
        union_vec(&mut self.sports, &other.sports);
        union_vec(&mut self.public_profile_urls, &other.public_profile_urls);
        union_vec(&mut self.source_identities, &other.source_identities);
        union_vec(&mut self.evidence, &other.evidence);
        for observation in other.observed_grades {
            if !self.observed_grades.contains(&observation) {
                self.observed_grades.push(observation);
            }
        }
        // Any observation that disagrees with the cohort lowers confidence instead of silently
        // rewriting the athlete's graduating class.
        if self
            .observed_grades
            .iter()
            .any(|observation| observation.grad_year() != self.grad_year)
        {
            self.identity_confidence = crate::model::Confidence::LOW;
        } else if self
            .observed_grades
            .iter()
            .any(|observation| observation.grad_year() == self.grad_year)
        {
            self.identity_confidence = crate::model::Confidence::HIGH;
        }
    }
}

impl Entity for CanonicalMeet {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        if self.location.is_none() {
            self.location = other.location;
        }
        if self.end_date.is_none() {
            self.end_date = other.end_date;
        }
        if self.level == crate::model::CompetitionLevel::Unknown {
            self.level = other.level;
        }
        union_vec(&mut self.sports, &other.sports);
        union_vec(&mut self.source_identities, &other.source_identities);
        union_vec(&mut self.source_urls, &other.source_urls);
        union_vec(&mut self.evidence, &other.evidence);
    }
}

impl Entity for CanonicalEvent {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        union_vec(&mut self.source_labels, &other.source_labels);
        union_vec(&mut self.evidence, &other.evidence);
    }
}

impl Entity for CanonicalPerformance {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        if self.wind_mps.is_none() {
            self.wind_mps = other.wind_mps;
        }
        if self.place.is_none() {
            self.place = other.place;
        }
        if self.observed_grade.is_none() {
            self.observed_grade = other.observed_grade;
        }
        if self.timing.is_none() {
            self.timing = other.timing;
        }
        union_vec(&mut self.evidence, &other.evidence);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn school(name: &str) -> CanonicalSchool {
        CanonicalSchool::new("WI", name, normalize_name(name)).0
    }

    #[test]
    fn consolidation_merges_evidence_and_identities() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let mut a = school("Abbotsford High School");
        a.source_identities
            .push(SourceIdentity::new(SourceNamespace::MilesplitTeam, "52649"));
        a.evidence.push(Evidence::parsed(
            SourceRef::id("milesplit_teams"),
            "2026-09-20",
        ));
        let mut b = a.clone();
        b.co_op = true;
        b.city = Some("Abbotsford".into());
        b.source_identities.push(SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: "wiaa".into(),
            },
            "1",
        ));
        store.append(Table::Schools, &a).unwrap();
        store.append(Table::Schools, &b).unwrap();
        let out = dir.path().join("out/schools.jsonl");
        let count = store
            .consolidate::<CanonicalSchool>(Table::Schools, &out)
            .unwrap();
        assert_eq!(count, 1);
        let merged: CanonicalSchool = serde_json::from_str(
            std::fs::read_to_string(&out)
                .unwrap()
                .lines()
                .next()
                .unwrap(),
        )
        .unwrap();
        assert!(merged.co_op);
        assert_eq!(merged.city.as_deref(), Some("Abbotsford"));
        assert_eq!(merged.source_identities.len(), 2);
    }

    #[test]
    fn journal_roundtrips_resume_keys() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        store
            .journal_done(
                "milesplit_rosters",
                "wi:52649",
                &serde_json::json!({"athletes": 59}),
            )
            .unwrap();
        store
            .journal_done(
                "milesplit_rosters",
                "wi:26848",
                &serde_json::json!({"athletes": 0}),
            )
            .unwrap();
        let keys = store.journal_keys("milesplit_rosters").unwrap();
        assert!(keys.contains("wi:52649"));
        assert_eq!(keys.len(), 2);
        assert_eq!(
            store.journal_payloads("milesplit_rosters").unwrap().len(),
            2
        );
    }
}
