use crate::DomainError;
use crate::sort::SpoolKey;
use std::collections::HashMap;
use std::path::Path;

/// Externalized athlete facts and candidate/provider incidence with global duplicate detection.
///
/// This module provides the foundation for globally detecting duplicate subjects, cases,
/// and verdict bindings across partitions, and building scheduling dependency components.

/// Represents an athlete fact that can be computed and stored externally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AthleteFact {
    /// The athlete identifier.
    pub athlete_id: String,
    /// The fact type (e.g., "name", "school", "class").
    pub fact_type: String,
    /// The fact value.
    pub fact_value: String,
    /// The source of the fact.
    pub source: String,
}

/// Represents a candidate/provider incidence edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncidenceEdge {
    /// The candidate identifier.
    pub candidate_id: String,
    /// The provider identifier.
    pub provider_id: String,
    /// The edge type (e.g., "primary", "link", "reference").
    pub edge_type: String,
}

/// Result of duplicate detection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateVerdict {
    /// The duplicate athlete IDs.
    pub duplicate_ids: Vec<String>,
    /// The duplicate fact type.
    pub fact_type: String,
    /// The duplicate values.
    pub duplicate_values: Vec<String>,
}

/// Externalized incidence manager.
pub struct IncidenceManager {
    /// In-memory cache of athlete facts.
    athlete_facts: HashMap<String, Vec<AthleteFact>>,
    /// In-memory cache of incidence edges.
    incidence_edges: HashMap<String, Vec<IncidenceEdge>>,
}

impl IncidenceManager {
    /// Create a new incidence manager.
    pub fn new() -> Self {
        Self {
            athlete_facts: HashMap::new(),
            incidence_edges: HashMap::new(),
        }
    }

    /// Add athlete facts.
    pub fn add_athlete_facts(&mut self, facts: &[AthleteFact]) -> Result<(), DomainError> {
        for fact in facts {
            self.athlete_facts
                .entry(fact.athlete_id.clone())
                .or_default()
                .push(fact.clone());
        }
        Ok(())
    }

    /// Add incidence edges.
    pub fn add_incidence_edges(&mut self, edges: &[IncidenceEdge]) -> Result<(), DomainError> {
        for edge in edges {
            self.incidence_edges
                .entry(edge.candidate_id.clone())
                .or_default()
                .push(edge.clone());
        }
        Ok(())
    }

    /// Detect duplicate subjects globally.
    pub fn detect_duplicate_subjects(&self) -> Vec<DuplicateVerdict> {
        let mut duplicates = vec![];

        for fact_type in ["name", "school", "class"] {
            let mut value_to_athletes: HashMap<String, Vec<String>> = HashMap::new();

            for (athlete_id, facts) in &self.athlete_facts {
                for fact in facts {
                    if fact.fact_type == fact_type {
                        value_to_athletes
                            .entry(fact.fact_value.clone())
                            .or_default()
                            .push(athlete_id.clone());
                    }
                }
            }

            for (value, athletes) in value_to_athletes {
                if athletes.len() > 1 {
                    duplicates.push(DuplicateVerdict {
                        duplicate_ids: athletes,
                        fact_type: fact_type.to_string(),
                        duplicate_values: vec![value],
                    });
                }
            }
        }

        duplicates
    }

    /// Build scheduling dependency components.
    pub fn build_dependency_components(&self) -> Vec<Vec<String>> {
        let mut components = vec![];
        let mut visited = HashMap::new();

        for (candidate_id, _) in &self.incidence_edges {
            if visited.get(candidate_id).cloned().unwrap_or(false) {
                continue;
            }

            let mut component = vec![];
            let mut queue = vec![candidate_id.clone()];

            while let Some(current) = queue.pop() {
                if visited.get(&current).cloned().unwrap_or(false) {
                    continue;
                }
                visited.insert(current.clone(), true);
                component.push(current.clone());

                if let Some(edges) = self.incidence_edges.get(&current) {
                    for edge in edges {
                        if !visited.get(&edge.provider_id).cloned().unwrap_or(false) {
                            queue.push(edge.provider_id.clone());
                        }
                    }
                }
            }

            if !component.is_empty() {
                components.push(component);
            }
        }

        components
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_athlete_facts() {
        let mut manager = IncidenceManager::new();
        let facts = vec![
            AthleteFact {
                athlete_id: "athlete1".to_string(),
                fact_type: "name".to_string(),
                fact_value: "John Doe".to_string(),
                source: "source1".to_string(),
            },
            AthleteFact {
                athlete_id: "athlete1".to_string(),
                fact_type: "school".to_string(),
                fact_value: "Lincoln High".to_string(),
                source: "source1".to_string(),
            },
        ];

        manager.add_athlete_facts(&facts).unwrap();
        assert_eq!(manager.athlete_facts["athlete1"].len(), 2);
    }

    #[test]
    fn test_add_incidence_edges() {
        let mut manager = IncidenceManager::new();
        let edges = vec![
            IncidenceEdge {
                candidate_id: "candidate1".to_string(),
                provider_id: "provider1".to_string(),
                edge_type: "primary".to_string(),
            },
        ];

        manager.add_incidence_edges(&edges).unwrap();
        assert_eq!(manager.incidence_edges["candidate1"].len(), 1);
    }

    #[test]
    fn test_detect_duplicate_subjects() {
        let mut manager = IncidenceManager::new();
        let facts = vec![
            AthleteFact {
                athlete_id: "athlete1".to_string(),
                fact_type: "name".to_string(),
                fact_value: "John Doe".to_string(),
                source: "source1".to_string(),
            },
            AthleteFact {
                athlete_id: "athlete2".to_string(),
                fact_type: "name".to_string(),
                fact_value: "John Doe".to_string(),
                source: "source2".to_string(),
            },
        ];

        manager.add_athlete_facts(&facts).unwrap();
        let duplicates = manager.detect_duplicate_subjects();
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0].duplicate_ids.len(), 2);
    }

    #[test]
    fn test_build_dependency_components() {
        let mut manager = IncidenceManager::new();
        let edges = vec![
            IncidenceEdge {
                candidate_id: "candidate1".to_string(),
                provider_id: "provider1".to_string(),
                edge_type: "primary".to_string(),
            },
            IncidenceEdge {
                candidate_id: "provider1".to_string(),
                provider_id: "provider2".to_string(),
                edge_type: "link".to_string(),
            },
        ];

        manager.add_incidence_edges(&edges).unwrap();
        let components = manager.build_dependency_components();
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].len(), 3);
    }
}