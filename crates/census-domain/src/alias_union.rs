use crate::DomainError;
use std::collections::{HashMap, HashSet};

/// Externalize accepted-alias union and ordered scope filters with excluded/unplaced streams.
///
/// This module provides the foundation for building the accepted-alias union from
/// projection results, maintaining ordered scope filters, and handling excluded
/// and unplaced streams separately.

/// Represents an alias relationship.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alias {
    /// The primary athlete identifier.
    pub primary_id: String,
    /// The alias athlete identifier.
    pub alias_id: String,
    /// The source of the alias relationship.
    pub source: String,
}

/// Represents a scope filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeFilter {
    /// Include all athletes.
    All,
    /// Include only athletes with specific properties.
    Filtered {
        property: String,
        values: HashSet<String>,
    },
    /// Exclude specific athletes.
    Excluded {
        excluded_ids: HashSet<String>,
    },
}

/// The alias union manager.
pub struct AliasUnionManager {
    /// Accepted aliases.
    accepted_aliases: HashMap<String, Vec<String>>,
    /// Unplaced athletes (not in any alias union).
    unplaced: HashSet<String>,
    /// Scope filters.
    filters: Vec<ScopeFilter>,
}

impl AliasUnionManager {
    pub fn new() -> Self {
        Self {
            accepted_aliases: HashMap::new(),
            unplaced: HashSet::new(),
            filters: vec![ScopeFilter::All],
        }
    }

    /// Add an alias relationship.
    pub fn add_alias(&mut self, alias: &Alias) -> Result<(), DomainError> {
        self.accepted_aliases
            .entry(alias.primary_id.clone())
            .or_default()
            .push(alias.alias_id.clone());
        Ok(())
    }

    /// Add an unplaced athlete.
    pub fn add_unplaced(&mut self, athlete_id: &str) -> Result<(), DomainError> {
        self.unplaced.insert(athlete_id.to_string());
        Ok(())
    }

    /// Add a scope filter.
    pub fn add_filter(&mut self, filter: ScopeFilter) -> Result<(), DomainError> {
        self.filters.push(filter);
        Ok(())
    }

    /// Get all accepted aliases for an athlete.
    pub fn get_aliases(&self, athlete_id: &str) -> &[String] {
        self.accepted_aliases
            .get(athlete_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Check if an athlete is unplaced.
    pub fn is_unplaced(&self, athlete_id: &str) -> bool {
        self.unplaced.contains(athlete_id)
    }

    /// Get all scope filters.
    pub fn filters(&self) -> &[ScopeFilter] {
        &self.filters
    }

    /// Apply all scope filters to a set of athletes.
    pub fn apply_filters(&self, athletes: &[String]) -> Vec<String> {
        let mut result = athletes.to_vec();

        for filter in &self.filters {
            match filter {
                ScopeFilter::All => {}
                ScopeFilter::Filtered { property, values } => {
                    // For simplicity, we don't have property access here
                    // This would be implemented with the actual domain objects
                    result.retain(|_| values.is_empty());
                }
                ScopeFilter::Excluded { excluded_ids } => {
                    result.retain(|id| !excluded_ids.contains(id));
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_alias() {
        let mut manager = AliasUnionManager::new();
        let alias = Alias {
            primary_id: "athlete1".to_string(),
            alias_id: "athlete1_alias".to_string(),
            source: "source1".to_string(),
        };

        manager.add_alias(&alias).unwrap();
        assert_eq!(
            manager.get_aliases("athlete1"),
            vec!["athlete1_alias".to_string()]
        );
    }

    #[test]
    fn test_add_unplaced() {
        let mut manager = AliasUnionManager::new();
        manager.add_unplaced("athlete2").unwrap();
        assert!(manager.is_unplaced("athlete2"));
        assert!(!manager.is_unplaced("athlete1"));
    }

    #[test]
    fn test_add_filter() {
        let mut manager = AliasUnionManager::new();
        manager.add_filter(ScopeFilter::All).unwrap();
        assert_eq!(manager.filters().len(), 2); // All + added
    }

    #[test]
    fn test_apply_filters() {
        let mut manager = AliasUnionManager::new();
        let mut excluded = HashSet::new();
        excluded.insert("athlete2".to_string());
        manager.add_filter(ScopeFilter::Excluded {
            excluded_ids: excluded,
        }).unwrap();

        let athletes = vec![
            "athlete1".to_string(),
            "athlete2".to_string(),
            "athlete3".to_string(),
        ];

        let filtered = manager.apply_filters(&athletes);
        assert_eq!(filtered.len(), 2);
        assert!(!filtered.contains(&"athlete2".to_string()));
    }
}