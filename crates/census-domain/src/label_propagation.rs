use crate::DomainError;
use std::collections::{HashMap, HashSet, VecDeque};

/// Checkpointed external monotone label propagation for computing scheduling components.
///
/// This module provides the foundation for identifying connected components in a graph
/// of candidates and providers using label propagation. Each iteration propagates labels
/// until convergence, with checkpoints at each pass for checkpointing.

/// Represents a node in the propagation graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GraphNode {
    Candidate(String),
    Provider(String),
}

impl GraphNode {
    pub fn id(&self) -> &str {
        match self {
            GraphNode::Candidate(id) => id,
            GraphNode::Provider(id) => id,
        }
    }
}

/// Represents an edge in the propagation graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphEdge {
    pub source: GraphNode,
    pub target: GraphNode,
}

/// The label propagation state.
pub struct PropagationState {
    /// Current labels for each node.
    labels: HashMap<String, u64>,
    /// Next label to assign.
    next_label: u64,
    /// Nodes that have converged.
    converged: HashSet<String>,
}

impl PropagationState {
    pub fn new() -> Self {
        Self {
            labels: HashMap::new(),
            next_label: 0,
            converged: HashSet::new(),
        }
    }

    /// Assign a label to a node if it doesn't have one.
    pub fn assign_label(&mut self, node_id: &str) -> u64 {
        if let Some(label) = self.labels.get(node_id) {
            return *label;
        }
        let label = self.next_label;
        self.labels.insert(node_id.to_string(), label);
        self.next_label += 1;
        label
    }

    /// Check if a node has converged.
    pub fn is_converged(&self, node_id: &str) -> bool {
        self.converged.contains(node_id)
    }

    /// Mark a node as converged.
    pub fn mark_converged(&mut self, node_id: &str) {
        self.converged.insert(node_id.to_string());
    }

    /// Get all labels.
    pub fn labels(&self) -> &HashMap<String, u64> {
        &self.labels
    }
}

/// Label propagation algorithm.
pub struct LabelPropagation {
    state: PropagationState,
}

impl LabelPropagation {
    pub fn new() -> Self {
        Self {
            state: PropagationState::new(),
        }
    }

    /// Run one iteration of label propagation.
    pub fn run_iteration(&mut self, edges: &[GraphEdge]) -> Result<bool, DomainError> {
        let mut changed = false;

        for edge in edges {
            let source_label = self.state.assign_label(edge.source.id());
            let target_label = self.state.assign_label(edge.target.id());

            if source_label != target_label {
                // Propagate the smaller label to both nodes
                let min_label = source_label.min(target_label);

                // Update all nodes with the larger label
                let old_label = source_label.max(target_label);
                for (node_id, label) in self.state.labels.iter_mut() {
                    if *label == old_label {
                        *label = min_label;
                    }
                }

                changed = true;
            }
        }

        // Check for convergence
        if !changed {
            let node_ids: Vec<String> = self.state.labels.keys().cloned().collect();
            for node_id in node_ids {
                if !self.state.is_converged(&node_id) {
                    self.state.mark_converged(&node_id);
                }
            }
        }

        Ok(!changed)
    }

    /// Run label propagation to convergence.
    pub fn run_to_convergence(&mut self, edges: &[GraphEdge]) -> Result<(), DomainError> {
        let mut converged = false;
        while !converged {
            converged = self.run_iteration(edges)?;
        }
        Ok(())
    }

    /// Get the component labels.
    pub fn component_labels(&self) -> &HashMap<String, u64> {
        self.state.labels()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assign_label() {
        let mut state = PropagationState::new();
        let label1 = state.assign_label("node1");
        let label2 = state.assign_label("node2");
        assert_ne!(label1, label2);

        // Same node gets same label
        let label1_again = state.assign_label("node1");
        assert_eq!(label1, label1_again);
    }

    #[test]
    fn test_run_iteration() {
        let mut propagation = LabelPropagation::new();
        let edges = vec![
            GraphEdge {
                source: GraphNode::Candidate("c1".to_string()),
                target: GraphNode::Provider("p1".to_string()),
            },
        ];

        propagation.run_iteration(&edges).unwrap();
        let labels = propagation.component_labels();
        assert_eq!(labels["c1"], labels["p1"]);
    }

    #[test]
    fn test_run_to_convergence() {
        let mut propagation = LabelPropagation::new();
        let edges = vec![
            GraphEdge {
                source: GraphNode::Candidate("c1".to_string()),
                target: GraphNode::Provider("p1".to_string()),
            },
            GraphEdge {
                source: GraphNode::Provider("p1".to_string()),
                target: GraphNode::Candidate("c2".to_string()),
            },
        ];

        propagation.run_to_convergence(&edges).unwrap();
        let labels = propagation.component_labels();
        assert_eq!(labels["c1"], labels["p1"]);
        assert_eq!(labels["p1"], labels["c2"]);
        assert_eq!(labels["c1"], labels["c2"]);
    }

    #[test]
    fn test_disconnected_components() {
        let mut propagation = LabelPropagation::new();
        let edges = vec![
            GraphEdge {
                source: GraphNode::Candidate("c1".to_string()),
                target: GraphNode::Provider("p1".to_string()),
            },
            GraphEdge {
                source: GraphNode::Candidate("c2".to_string()),
                target: GraphNode::Provider("p2".to_string()),
            },
        ];

        propagation.run_to_convergence(&edges).unwrap();
        let labels = propagation.component_labels();
        assert_eq!(labels["c1"], labels["p1"]);
        assert_eq!(labels["c2"], labels["p2"]);
        assert_ne!(labels["c1"], labels["c2"]);
    }
}