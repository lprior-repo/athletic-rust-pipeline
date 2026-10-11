use crate::DomainError;
use crate::sort::SpoolKey;
use std::collections::HashMap;

/// Run the existing projection builder per budgeted component and validate against the global oracle.
///
/// This module provides the foundation for building identity projections from candidate
/// components, running the projection builder for each component, and validating the
/// resulting projections against a global oracle.

/// Represents a projection component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionComponent {
    /// The component identifier.
    pub component_id: u64,
    /// The candidate identifiers in this component.
    pub candidate_ids: Vec<String>,
}

/// Represents a projection result.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectionResult {
    /// The projected athlete identifier.
    pub athlete_id: String,
    /// The source candidate identifiers.
    pub source_candidates: Vec<String>,
    /// The projection confidence.
    pub confidence: f64,
}

/// The global oracle for validation.
pub struct GlobalOracle {
    /// Known projections for validation.
    projections: HashMap<String, ProjectionResult>,
}

impl GlobalOracle {
    pub fn new() -> Self {
        Self {
            projections: HashMap::new(),
        }
    }

    /// Register a known projection.
    pub fn register_projection(&mut self, projection: ProjectionResult) -> Result<(), DomainError> {
        self.projections.insert(projection.athlete_id.clone(), projection);
        Ok(())
    }

    /// Validate a projection against the oracle.
    pub fn validate_projection(&self, projection: &ProjectionResult) -> Result<(), DomainError> {
        if let Some(expected) = self.projections.get(&projection.athlete_id) {
            if expected != projection {
                return Err(DomainError::OutOfRange {
                    field: "projection validation",
                });
            }
        }
        Ok(())
    }
}

/// Projection builder.
pub struct ProjectionBuilder {
    budget: u64,
}

impl ProjectionBuilder {
    /// Create a new projection builder with a budget.
    pub fn new(budget: u64) -> Self {
        Self { budget }
    }

    /// Build a projection for a component.
    pub fn build_projection(
        &self,
        component: &ProjectionComponent,
    ) -> Result<ProjectionResult, DomainError> {
        // Simple projection: the first candidate becomes the canonical athlete
        if component.candidate_ids.is_empty() {
            return Err(DomainError::OutOfRange {
                field: "candidate count",
            });
        }

        let athlete_id = component.candidate_ids[0].clone();
        Ok(ProjectionResult {
            athlete_id,
            source_candidates: component.candidate_ids.clone(),
            confidence: 1.0,
        })
    }

    /// Validate a projection against the global oracle.
    pub fn validate(&self, projection: &ProjectionResult, oracle: &GlobalOracle) -> Result<(), DomainError> {
        oracle.validate_projection(projection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_projection() {
        let builder = ProjectionBuilder::new(1000);
        let component = ProjectionComponent {
            component_id: 1,
            candidate_ids: vec!["candidate1".to_string(), "candidate2".to_string()],
        };

        let projection = builder.build_projection(&component).unwrap();
        assert_eq!(projection.athlete_id, "candidate1");
        assert_eq!(projection.source_candidates.len(), 2);
    }

    #[test]
    fn test_build_projection_empty() {
        let builder = ProjectionBuilder::new(1000);
        let component = ProjectionComponent {
            component_id: 1,
            candidate_ids: vec![],
        };

        let result = builder.build_projection(&component);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_projection() {
        let builder = ProjectionBuilder::new(1000);
        let component = ProjectionComponent {
            component_id: 1,
            candidate_ids: vec!["candidate1".to_string()],
        };

        let projection = builder.build_projection(&component).unwrap();

        let mut oracle = GlobalOracle::new();
        oracle.register_projection(projection.clone()).unwrap();
        builder.validate(&projection, &oracle).unwrap();
    }

    #[test]
    fn test_validate_projection_mismatch() {
        let builder = ProjectionBuilder::new(1000);
        let component = ProjectionComponent {
            component_id: 1,
            candidate_ids: vec!["candidate1".to_string()],
        };

        let projection = builder.build_projection(&component).unwrap();

        let mut oracle = GlobalOracle::new();
        let wrong_projection = ProjectionResult {
            athlete_id: "candidate1".to_string(),
            source_candidates: vec!["wrong_candidate".to_string()],
            confidence: 1.0,
        };
        oracle.register_projection(wrong_projection).unwrap();

        let result = builder.validate(&projection, &oracle);
        assert!(result.is_err());
    }
}