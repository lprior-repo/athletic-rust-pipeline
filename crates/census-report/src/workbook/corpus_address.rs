use crate::report::{ReportError, ReportResult};
use census_domain::model::{CanonicalAthlete, CanonicalSchool};
use census_domain::school_directory::{DirectoryIndex, LinkDecision, SchoolDirectoryEntry};
use census_domain::UsJurisdiction;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) struct CorpusAddresses {
    index: DirectoryIndex,
    digest: String,
    rows: usize,
}

impl CorpusAddresses {
    pub(crate) fn load(dir: &Path) -> ReportResult<Self> {
        let path = dir.join("school_directory.json");
        let raw = std::fs::read(&path).map_err(|source| ReportError::Invariant {
            detail: format!("address corpus {} unreadable: {source}", path.display()),
        })?;
        let digest = format!("{:x}", Sha256::digest(&raw));
        let entries: Vec<SchoolDirectoryEntry> =
            serde_json::from_slice(&raw).map_err(|source| ReportError::Invariant {
                detail: format!(
                    "address corpus {} is not directory JSON: {source}",
                    path.display()
                ),
            })?;
        let rows = entries.len();
        Ok(Self {
            index: DirectoryIndex::build(&entries),
            digest,
            rows,
        })
    }

    pub(crate) fn digest(&self) -> &str {
        &self.digest
    }

    pub(crate) fn rows(&self) -> usize {
        self.rows
    }

    pub(crate) fn fill_gaps(
        &self,
        map: &mut BTreeMap<String, String>,
        athletes: &[CanonicalAthlete],
        schools: &BTreeMap<&str, &CanonicalSchool>,
    ) -> Vec<QuarantineRow> {
        let mut quarantine: BTreeMap<String, QuarantineRow> = BTreeMap::new();
        for athlete in athletes {
            if map.contains_key(athlete.id.as_str()) {
                continue;
            }
            let Some(school) = schools
                .get(athlete.school.as_str())
                .copied()
                .map(SchoolProbe::of_canonical)
            else {
                continue;
            };
            let entry = quarantine
                .entry(school.id.to_owned())
                .or_insert_with(|| QuarantineRow::base(&school));
            entry.athletes_affected += 1;
            if entry.reasoned() {
                continue;
            }
            let Some(state) = school.state else {
                entry.set_reason("missing_state");
                continue;
            };
            match self.index.link(
                school.name,
                school.normalized,
                school.city,
                state,
                school.aliases,
                &[],
            ) {
                LinkDecision::Linked(matched) => {
                    if let Some(line) = render_address(matched.address.as_ref(), &school, state) {
                        map.insert(athlete.id.as_str().to_owned(), line);
                        quarantine.remove(school.id);
                    } else {
                        entry.set_reason("linked_but_no_street");
                    }
                }
                LinkDecision::Review { reason, candidates } => {
                    entry.set_ambiguous(reason, candidates);
                }
                LinkDecision::NoMatch => {
                    entry.set_reason("no_match");
                }
            }
        }
        quarantine.into_values().collect()
    }
}

pub(crate) struct SchoolProbe<'a> {
    pub(crate) id: &'a str,
    pub(crate) name: &'a str,
    pub(crate) normalized: &'a str,
    pub(crate) city: Option<&'a str>,
    pub(crate) state: Option<UsJurisdiction>,
    pub(crate) aliases: &'a [String],
}

impl<'a> SchoolProbe<'a> {
    pub(crate) fn of_canonical(school: &'a CanonicalSchool) -> Self {
        Self {
            id: school.id.as_str(),
            name: school.name.as_str(),
            normalized: school.normalized_name.as_str(),
            city: school.city.as_deref(),
            state: school.state,
            aliases: school.aliases.as_slice(),
        }
    }
}

fn render_address(
    address: Option<&census_domain::school_directory::PostalAddress>,
    school: &SchoolProbe<'_>,
    state: UsJurisdiction,
) -> Option<String> {
    let address = address?;
    let street = address.line1()?.as_str().trim();
    if street.is_empty() {
        return None;
    }
    let city = address
        .city()
        .map(|city| city.as_str())
        .or(school.city)
        .unwrap_or("")
        .trim();
    let state_code = state.code();
    let mut line = String::from(street);
    if !city.is_empty() {
        line.push_str(", ");
        line.push_str(city);
    }
    line.push(' ');
    line.push_str(state_code);
    if let Some(zip) = address.zip() {
        line.push(' ');
        line.push_str(zip.code());
        if let Some(plus4) = zip.plus4() {
            line.push('-');
            line.push_str(plus4);
        }
    }
    Some(line)
}

#[derive(Debug, Clone)]
pub(crate) struct QuarantineRow {
    pub(crate) school_id: String,
    pub(crate) school_name: String,
    pub(crate) city: String,
    pub(crate) state: String,
    pub(crate) reason: &'static str,
    pub(crate) candidates: String,
    pub(crate) athletes_affected: usize,
}

impl QuarantineRow {
    fn base(school: &SchoolProbe<'_>) -> Self {
        Self {
            school_id: school.id.to_owned(),
            school_name: school.name.to_owned(),
            city: school.city.unwrap_or_default().to_owned(),
            state: school
                .state
                .map(|state| state.code().to_owned())
                .unwrap_or_default(),
            reason: "",
            candidates: String::new(),
            athletes_affected: 0,
        }
    }

    fn reasoned(&self) -> bool {
        !self.reason.is_empty()
    }

    fn set_reason(&mut self, reason: &'static str) {
        self.reason = reason;
    }

    fn set_ambiguous(
        &mut self,
        reason: census_domain::school_directory::ReviewReason,
        candidates: Vec<census_domain::school_directory::CandidateRef>,
    ) {
        self.reason = match reason {
            census_domain::school_directory::ReviewReason::Ambiguous => "ambiguous",
        };
        let mut names: Vec<String> = candidates
            .iter()
            .map(|candidate| candidate.key.label())
            .collect();
        names.sort();
        names.dedup();
        names.truncate(5);
        self.candidates = names.join(" | ");
    }
}

pub(crate) fn write_quarantine_csv(
    path: &Path,
    digest: &str,
    rows: &[QuarantineRow],
) -> ReportResult<()> {
    let mut out = csv_line(&[
        "school_id",
        "school_name",
        "city",
        "state",
        "reason",
        "candidates",
        "athletes_affected",
        "corpus_digest",
    ]);
    for row in rows {
        let affected = row.athletes_affected.to_string();
        out.push_str(&csv_line(&[
            row.school_id.as_str(),
            row.school_name.as_str(),
            row.city.as_str(),
            row.state.as_str(),
            row.reason,
            row.candidates.as_str(),
            affected.as_str(),
            digest,
        ]));
    }
    std::fs::write(path, out).map_err(|source| ReportError::Invariant {
        detail: format!("address quarantine {} unwritable: {source}", path.display()),
    })?;
    Ok(())
}

fn csv_line(fields: &[&str]) -> String {
    let mut line = String::new();
    for (position, field) in fields.iter().enumerate() {
        if position > 0 {
            line.push(',');
        }
        line.push('"');
        line.push_str(field.replace('"', "\"\"").as_str());
        line.push('"');
    }
    line.push('\n');
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use census_domain::model::{Gender, GradYear, SourceIdentity, SourceNamespace};
    use census_domain::school_directory::{
        CityName, NcesSchoolId, PostalAddress, SchoolName, SourceLabel, StreetLine, ZipCode,
    };
    use std::collections::BTreeMap;

    const CORPUS_DIGEST: &str = "fixture-digest";

    fn alabama() -> UsJurisdiction {
        UsJurisdiction::Alabama
    }

    fn entry(nces: &str, name: &str, street: Option<&str>, city: &str) -> SchoolDirectoryEntry {
        let address = PostalAddress::of(
            street.map(|line| StreetLine::parse(line).expect("fixture street")),
            None,
            Some(CityName::parse(city).expect("fixture city")),
            Some(alabama()),
            Some(ZipCode::of("35950", Some("2336")).expect("fixture zip")),
        )
        .expect("fixture address");
        SchoolDirectoryEntry::identified(
            census_domain::school_directory::IdentifiedKey::Nces(
                NcesSchoolId::parse(nces).expect("fixture nces id"),
            ),
            SourceLabel::Ccd,
            Some(SchoolName::parse(name).expect("fixture school name")),
        )
        .with_jurisdiction(Some(alabama()))
        .with_address(Some(address))
    }

    fn school_map(schools: &[CanonicalSchool]) -> BTreeMap<&str, &CanonicalSchool> {
        schools
            .iter()
            .map(|school| (school.id.as_str(), school))
            .collect()
    }

    fn athlete_at(school: &CanonicalSchool, name: &str) -> CanonicalAthlete {
        CanonicalAthlete::new(
            &school.id,
            name,
            GradYear::CO2027,
            Gender::Boys,
            SourceIdentity::new(SourceNamespace::Other("fixture".to_owned()), name),
        )
        .expect("fixture athlete")
    }

    fn owned_school(name: &str, city: Option<&str>) -> CanonicalSchool {
        let (school, _) = CanonicalSchool::new(alabama(), name, name, city);
        school
    }

    fn corpus(entries: &[SchoolDirectoryEntry]) -> CorpusAddresses {
        CorpusAddresses {
            index: DirectoryIndex::build(entries),
            digest: CORPUS_DIGEST.to_owned(),
            rows: entries.len(),
        }
    }

    #[test]
    fn loader_rejects_missing_directory_file() {
        let missing = Path::new("/definitely/not/a/corpus/dir");
        let result = CorpusAddresses::load(missing);
        assert!(matches!(result, Err(ReportError::Invariant { .. })));
        let detail = match result {
            Err(ReportError::Invariant { detail }) => detail,
            _ => unreachable!(),
        };
        assert!(detail.contains("school_directory.json"));
    }

    #[test]
    fn loader_rejects_malformed_directory_json() {
        let dir = tempfile::tempdir().expect("fixture tempdir");
        std::fs::write(dir.path().join("school_directory.json"), b"{nope").expect("fixture write");
        let result = CorpusAddresses::load(dir.path());
        assert!(matches!(result, Err(ReportError::Invariant { .. })));
    }

    #[test]
    fn lane_wire_shape_deserializes_to_directory_entries() {
        let raw = r#"[{
            "key": {"Nces": "010000500870"},
            "name": "Albertville Middle School",
            "address": {
                "line1": "600 E Alabama Ave",
                "city": "Albertville",
                "state": "AL",
                "zip": {"code": "35950", "plus4": "2336"}
            },
            "sources": ["Ccd"],
            "provenance": {"Name": "Ccd", "Address": "Ccd"}
        }]"#;
        let entries: Vec<SchoolDirectoryEntry> =
            serde_json::from_str(raw).expect("lane wire shape parses");
        assert_eq!(entries.len(), 1);
        let name = entries[0].name().expect("entry has a name");
        assert_eq!(name.as_str(), "Albertville Middle School");
        let address = entries[0].address().expect("entry has an address");
        assert_eq!(
            address.line1().expect("street").as_str(),
            "600 E Alabama Ave"
        );
    }

    #[test]
    fn fill_gaps_writes_formatted_address_when_corpus_links() {
        let addresses = corpus(&[entry(
            "010000500870",
            "Albertville High School",
            Some("600 E Alabama Ave"),
            "Albertville",
        )]);
        let school = owned_school("Albertville High School", Some("Albertville"));
        let athletes = vec![athlete_at(&school, "Ava Runner")];
        let mut map = BTreeMap::new();
        let quarantine = addresses.fill_gaps(&mut map, &athletes, &school_map(&[school]));
        assert!(quarantine.is_empty());
        assert_eq!(map.len(), 1);
        let line = map.values().next().expect("one filled address");
        assert_eq!(line, "600 E Alabama Ave, Albertville AL 35950-2336");
    }

    #[test]
    fn fill_gaps_prefers_store_claim_over_corpus() {
        let addresses = corpus(&[entry(
            "010000500870",
            "Albertville High School",
            Some("600 E Alabama Ave"),
            "Albertville",
        )]);
        let school = owned_school("Albertville High School", Some("Albertville"));
        let athletes = vec![athlete_at(&school, "Ava Runner")];
        let id = athletes[0].id.as_str().to_owned();
        let mut map = BTreeMap::from([(id.clone(), "9 Claim St, Claimton AL 00000".to_owned())]);
        let quarantine = addresses.fill_gaps(&mut map, &athletes, &school_map(&[school]));
        assert!(quarantine.is_empty());
        assert_eq!(
            map.get(&id).map(String::as_str),
            Some("9 Claim St, Claimton AL 00000")
        );
    }

    #[test]
    fn fill_gaps_quarantines_school_absent_from_corpus() {
        let addresses = corpus(&[entry(
            "010000500870",
            "Albertville High School",
            Some("600 E Alabama Ave"),
            "Albertville",
        )]);
        let school = owned_school("Nowhere High School", Some("Nowhere"));
        let athletes = vec![athlete_at(&school, "Nia Lost")];
        let mut map = BTreeMap::new();
        let quarantine = addresses.fill_gaps(&mut map, &athletes, &school_map(&[school]));
        assert!(map.is_empty());
        assert_eq!(quarantine.len(), 1);
        assert_eq!(quarantine[0].reason, "no_match");
        assert_eq!(quarantine[0].school_name, "Nowhere High School");
        assert_eq!(quarantine[0].athletes_affected, 1);
    }

    #[test]
    fn fill_gaps_dedupes_quarantine_rows_per_school() {
        let addresses = corpus(&[entry(
            "010000500870",
            "Albertville High School",
            Some("600 E Alabama Ave"),
            "Albertville",
        )]);
        let school = owned_school("Nowhere High School", Some("Nowhere"));
        let athletes = vec![
            athlete_at(&school, "Nia Lost"),
            athlete_at(&school, "Noah Lost"),
        ];
        let mut map = BTreeMap::new();
        let quarantine = addresses.fill_gaps(&mut map, &athletes, &school_map(&[school]));
        assert!(map.is_empty());
        assert_eq!(quarantine.len(), 1);
        assert_eq!(quarantine[0].reason, "no_match");
        assert_eq!(quarantine[0].athletes_affected, 2);
    }

    #[test]
    fn fill_gaps_quarantines_ambiguous_matches_with_candidates() {
        let addresses = corpus(&[
            entry(
                "010000500870",
                "Roosevelt High School",
                Some("1 A St"),
                "Birmingham",
            ),
            entry(
                "010000500871",
                "Roosevelt High School",
                Some("2 B St"),
                "Birmingham",
            ),
        ]);
        let school = owned_school("Roosevelt High School", Some("Birmingham"));
        let athletes = vec![athlete_at(&school, "Ari Tied")];
        let mut map = BTreeMap::new();
        let quarantine = addresses.fill_gaps(&mut map, &athletes, &school_map(&[school]));
        assert!(map.is_empty());
        assert_eq!(quarantine.len(), 1);
        assert_eq!(quarantine[0].reason, "ambiguous");
        assert!(quarantine[0].candidates.contains("nces:010000500870"));
        assert!(quarantine[0].candidates.contains("nces:010000500871"));
    }

    #[test]
    fn fill_gaps_quarantines_school_missing_state() {
        let addresses = corpus(&[entry(
            "010000500870",
            "Albertville High School",
            Some("600 E Alabama Ave"),
            "Albertville",
        )]);
        let (mut school, _) =
            CanonicalSchool::new(alabama(), "Stateless High", "Stateless High", None);
        school.state = None;
        let athletes = vec![athlete_at(&school, "Sam Stateless")];
        let mut map = BTreeMap::new();
        let quarantine = addresses.fill_gaps(&mut map, &athletes, &school_map(&[school]));
        assert!(map.is_empty());
        assert_eq!(quarantine.len(), 1);
        assert_eq!(quarantine[0].reason, "missing_state");
    }

    #[test]
    fn fill_gaps_quarantines_link_without_street() {
        let addresses = corpus(&[entry(
            "010000500870",
            "Streetless High School",
            None,
            "Albertville",
        )]);
        let school = owned_school("Streetless High School", Some("Albertville"));
        let athletes = vec![athlete_at(&school, "Stu Nums")];
        let mut map = BTreeMap::new();
        let quarantine = addresses.fill_gaps(&mut map, &athletes, &school_map(&[school]));
        assert!(map.is_empty());
        assert_eq!(quarantine.len(), 1);
        assert_eq!(quarantine[0].reason, "linked_but_no_street");
    }

    #[test]
    fn quarantine_csv_quotes_commas_and_quotes() {
        let dir = tempfile::tempdir().expect("fixture tempdir");
        let path = dir.path().join("address-quarantine.csv");
        let rows = vec![QuarantineRow {
            school_id: "sch-1".to_owned(),
            school_name: "St. \"Oaks\", High".to_owned(),
            city: "Birmingham".to_owned(),
            state: "AL".to_owned(),
            reason: "ambiguous",
            candidates: "nces:1 | nces:2".to_owned(),
            athletes_affected: 41,
        }];
        write_quarantine_csv(&path, CORPUS_DIGEST, &rows).expect("csv writes");
        let body = std::fs::read_to_string(&path).expect("csv reads back");
        assert_eq!(
            body,
            "\"school_id\",\"school_name\",\"city\",\"state\",\"reason\",\"candidates\",\"athletes_affected\",\"corpus_digest\"\n\"sch-1\",\"St. \"\"Oaks\"\", High\",\"Birmingham\",\"AL\",\"ambiguous\",\"nces:1 | nces:2\",\"41\",\"fixture-digest\"\n"
        );
    }
}
