use crate::DomainError;
use std::collections::{HashMap, HashSet};

/// Externalize the context-first PR reducer, same-course keys and conflict/distinct spools.
///
/// This module provides the foundation for reducing personal record (PR) performance data,
/// resolving conflicts, and producing distinct spools for publication.

/// Represents a PR performance.
#[derive(Debug, Clone, PartialEq)]
pub struct PRPerformance {
    /// The performance identifier.
    pub performance_id: String,
    /// The athlete identifier.
    pub athlete_id: String,
    /// The course/event name.
    pub course: String,
    /// The performance value.
    pub value: f64,
    /// The date of the performance.
    pub date: String,
}

/// Represents a PR context (course, conditions).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PRContext {
    /// The course name.
    pub course: String,
    /// The conditions (e.g., indoor/outdoor, wind-legal).
    pub conditions: String,
}

/// Represents a PR reducer result.
#[derive(Debug, Clone, PartialEq)]
pub struct PRResult {
    /// The athlete identifier.
    pub athlete_id: String,
    /// The PR context.
    pub context: PRContext,
    /// The best performance value.
    pub best_value: f64,
    /// The date of the best performance.
    pub best_date: String,
    /// The performance ID of the best performance.
    pub best_performance_id: String,
}

/// Represents a conflict between PR performances.
#[derive(Debug, Clone, PartialEq)]
pub struct PRConflict {
    /// The athlete identifier.
    pub athlete_id: String,
    /// The PR context.
    pub context: PRContext,
    /// The conflicting performances.
    pub performances: Vec<PRPerformance>,
}

/// The PR reducer manager.
pub struct PRReducerManager {
    /// Best PRs per athlete/context.
    best_prs: HashMap<(String, PRContext), PRResult>,
    /// Conflicts to resolve.
    conflicts: Vec<PRConflict>,
}

impl PRReducerManager {
    pub fn new() -> Self {
        Self {
            best_prs: HashMap::new(),
            conflicts: vec![],
        }
    }

    /// Add a PR performance.
    pub fn add_performance(&mut self, perf: PRPerformance) -> Result<(), DomainError> {
        let context = PRContext {
            course: perf.course.clone(),
            conditions: "standard".to_string(),
        };

        let key = (perf.athlete_id.clone(), context.clone());

        if let Some(existing) = self.best_prs.get(&key) {
            // Compare values (lower is better for time events)
            if perf.value < existing.best_value {
                // New best PR
                self.best_prs.insert(key.clone(), PRResult {
                    athlete_id: perf.athlete_id.clone(),
                    context: context.clone(),
                    best_value: perf.value,
                    best_date: perf.date.clone(),
                    best_performance_id: perf.performance_id.clone(),
                });
            } else if perf.value == existing.best_value {
                // Tie: record conflict
                self.conflicts.push(PRConflict {
                    athlete_id: perf.athlete_id.clone(),
                    context: context.clone(),
                    performances: vec![
                        PRPerformance {
                            performance_id: existing.best_performance_id.clone(),
                            athlete_id: existing.athlete_id.clone(),
                            course: existing.context.course.clone(),
                            value: existing.best_value,
                            date: existing.best_date.clone(),
                        },
                        perf,
                    ],
                });
            }
        } else {
            // First PR for this context
            self.best_prs.insert(key, PRResult {
                athlete_id: perf.athlete_id.clone(),
                context: context.clone(),
                best_value: perf.value,
                best_date: perf.date.clone(),
                best_performance_id: perf.performance_id.clone(),
            });
        }

        Ok(())
    }

    /// Get the best PRs for an athlete.
    pub fn get_athlete_prs(&self, athlete_id: &str) -> Vec<&PRResult> {
        self.best_prs
            .values()
            .filter(|pr| pr.athlete_id == athlete_id)
            .collect()
    }

    /// Get all conflicts.
    pub fn get_conflicts(&self) -> &[PRConflict] {
        &self.conflicts
    }

    /// Get all distinct PR results.
    pub fn get_distinct_prs(&self) -> Vec<&PRResult> {
        self.best_prs.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_performance_first_pr() {
        let mut manager = PRReducerManager::new();
        let perf = PRPerformance {
            performance_id: "perf1".to_string(),
            athlete_id: "athlete1".to_string(),
            course: "100m".to_string(),
            value: 10.5,
            date: "2026-10-01".to_string(),
        };

        manager.add_performance(perf).unwrap();
        let prs = manager.get_athlete_prs("athlete1");
        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].best_value, 10.5);
    }

    #[test]
    fn test_add_performance_new_best() {
        let mut manager = PRReducerManager::new();
        let perf1 = PRPerformance {
            performance_id: "perf1".to_string(),
            athlete_id: "athlete1".to_string(),
            course: "100m".to_string(),
            value: 10.5,
            date: "2026-10-01".to_string(),
        };
        manager.add_performance(perf1).unwrap();

        let perf2 = PRPerformance {
            performance_id: "perf2".to_string(),
            athlete_id: "athlete1".to_string(),
            course: "100m".to_string(),
            value: 10.4,
            date: "2026-10-02".to_string(),
        };
        manager.add_performance(perf2).unwrap();

        let prs = manager.get_athlete_prs("athlete1");
        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].best_value, 10.4);
    }

    #[test]
    fn test_add_performance_tie() {
        let mut manager = PRReducerManager::new();
        let perf1 = PRPerformance {
            performance_id: "perf1".to_string(),
            athlete_id: "athlete1".to_string(),
            course: "100m".to_string(),
            value: 10.5,
            date: "2026-10-01".to_string(),
        };
        manager.add_performance(perf1).unwrap();

        let perf2 = PRPerformance {
            performance_id: "perf2".to_string(),
            athlete_id: "athlete1".to_string(),
            course: "100m".to_string(),
            value: 10.5,
            date: "2026-10-02".to_string(),
        };
        manager.add_performance(perf2).unwrap();

        let conflicts = manager.get_conflicts();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].performances.len(), 2);
    }

    #[test]
    fn test_distinct_prs() {
        let mut manager = PRReducerManager::new();
        let perf1 = PRPerformance {
            performance_id: "perf1".to_string(),
            athlete_id: "athlete1".to_string(),
            course: "100m".to_string(),
            value: 10.5,
            date: "2026-10-01".to_string(),
        };
        manager.add_performance(perf1).unwrap();

        let perf2 = PRPerformance {
            performance_id: "perf2".to_string(),
            athlete_id: "athlete1".to_string(),
            course: "200m".to_string(),
            value: 21.0,
            date: "2026-10-01".to_string(),
        };
        manager.add_performance(perf2).unwrap();

        let distinct = manager.get_distinct_prs();
        assert_eq!(distinct.len(), 2);
    }
}