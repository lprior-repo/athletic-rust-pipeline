use crate::DomainError;
use crate::sort::SpoolKey;
use std::collections::HashMap;

/// External-join and sort enriched performance rows into the exact display order with bounded partitions.
///
/// This module provides the foundation for joining performance data with enriched athlete
/// information, sorting the joined results into the exact display order, and partitioning
/// the results into bounded size chunks.

/// Represents a performance record.
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceRecord {
    /// The performance identifier.
    pub performance_id: String,
    /// The athlete identifier.
    pub athlete_id: String,
    /// The event name.
    pub event_name: String,
    /// The performance value.
    pub value: f64,
    /// The date of the performance.
    pub date: String,
}

/// Represents an enriched performance row.
#[derive(Debug, Clone, PartialEq)]
pub struct EnrichedPerformanceRow {
    /// The performance data.
    pub performance: PerformanceRecord,
    /// The athlete's name.
    pub athlete_name: String,
    /// The athlete's school.
    pub school: String,
    /// The athlete's class year.
    pub class_year: u16,
}

/// Represents a display order specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisplayOrder {
    ByDate,
    ByValue,
    ByEventThenValue,
}

/// The performance join manager.
pub struct PerformanceJoinManager {
    /// Athlete information for enrichment.
    athlete_info: HashMap<String, (String, String, u16)>,
}

impl PerformanceJoinManager {
    pub fn new() -> Self {
        Self {
            athlete_info: HashMap::new(),
        }
    }

    /// Add athlete information for enrichment.
    pub fn add_athlete_info(
        &mut self,
        athlete_id: &str,
        name: &str,
        school: &str,
        class_year: u16,
    ) -> Result<(), DomainError> {
        self.athlete_info.insert(
            athlete_id.to_string(),
            (name.to_string(), school.to_string(), class_year),
        );
        Ok(())
    }

    /// Join performance records with athlete information.
    pub fn join_performances(&self, performances: &[PerformanceRecord]) -> Vec<EnrichedPerformanceRow> {
        let mut enriched = vec![];

        for perf in performances {
            if let Some((name, school, class_year)) = self.athlete_info.get(&perf.athlete_id) {
                enriched.push(EnrichedPerformanceRow {
                    performance: perf.clone(),
                    athlete_name: name.clone(),
                    school: school.clone(),
                    class_year: *class_year,
                });
            }
        }

        enriched
    }

    /// Sort enriched rows into the exact display order.
    pub fn sort_for_display(&self, rows: &[EnrichedPerformanceRow], order: &DisplayOrder) -> Vec<EnrichedPerformanceRow> {
        let mut sorted = rows.to_vec();

        match order {
            DisplayOrder::ByDate => {
                sorted.sort_by(|a, b| a.performance.date.cmp(&b.performance.date));
            }
            DisplayOrder::ByValue => {
                sorted.sort_by(|a, b| b.performance.value.partial_cmp(&a.performance.value).unwrap_or(std::cmp::Ordering::Equal));
            }
            DisplayOrder::ByEventThenValue => {
                sorted.sort_by(|a, b| {
                    let event_cmp = a.performance.event_name.cmp(&b.performance.event_name);
                    if event_cmp != std::cmp::Ordering::Equal {
                        return event_cmp;
                    }
                    b.performance.value.partial_cmp(&a.performance.value).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }

        sorted
    }

    /// Partition sorted rows into bounded size chunks.
    pub fn partition_rows(&self, rows: &[EnrichedPerformanceRow], partition_size: usize) -> Vec<Vec<EnrichedPerformanceRow>> {
        let mut partitions = vec![];
        let mut current_partition = vec![];

        for row in rows {
            current_partition.push(row.clone());
            if current_partition.len() == partition_size {
                partitions.push(current_partition);
                current_partition = vec![];
            }
        }

        if !current_partition.is_empty() {
            partitions.push(current_partition);
        }

        partitions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_performances() {
        let mut manager = PerformanceJoinManager::new();
        manager
            .add_athlete_info("athlete1", "John Doe", "Lincoln High", 2027)
            .unwrap();

        let performances = vec![PerformanceRecord {
            performance_id: "perf1".to_string(),
            athlete_id: "athlete1".to_string(),
            event_name: "100m".to_string(),
            value: 10.5,
            date: "2026-10-01".to_string(),
        }];

        let enriched = manager.join_performances(&performances);
        assert_eq!(enriched.len(), 1);
        assert_eq!(enriched[0].athlete_name, "John Doe");
    }

    #[test]
    fn test_sort_by_date() {
        let mut manager = PerformanceJoinManager::new();
        manager
            .add_athlete_info("athlete1", "John Doe", "Lincoln High", 2027)
            .unwrap();

        let performances = vec![
            PerformanceRecord {
                performance_id: "perf1".to_string(),
                athlete_id: "athlete1".to_string(),
                event_name: "100m".to_string(),
                value: 10.5,
                date: "2026-10-02".to_string(),
            },
            PerformanceRecord {
                performance_id: "perf2".to_string(),
                athlete_id: "athlete1".to_string(),
                event_name: "100m".to_string(),
                value: 10.6,
                date: "2026-10-01".to_string(),
            },
        ];

        let enriched = manager.join_performances(&performances);
        let sorted = manager.sort_for_display(&enriched, &DisplayOrder::ByDate);
        assert_eq!(sorted[0].performance.date, "2026-10-01");
        assert_eq!(sorted[1].performance.date, "2026-10-02");
    }

    #[test]
    fn test_partition_rows() {
        let mut manager = PerformanceJoinManager::new();
        manager
            .add_athlete_info("athlete1", "John Doe", "Lincoln High", 2027)
            .unwrap();

        let performances = vec![
            PerformanceRecord {
                performance_id: "perf1".to_string(),
                athlete_id: "athlete1".to_string(),
                event_name: "100m".to_string(),
                value: 10.5,
                date: "2026-10-01".to_string(),
            },
            PerformanceRecord {
                performance_id: "perf2".to_string(),
                athlete_id: "athlete1".to_string(),
                event_name: "200m".to_string(),
                value: 21.0,
                date: "2026-10-01".to_string(),
            },
            PerformanceRecord {
                performance_id: "perf3".to_string(),
                athlete_id: "athlete1".to_string(),
                event_name: "400m".to_string(),
                value: 45.0,
                date: "2026-10-01".to_string(),
            },
        ];

        let enriched = manager.join_performances(&performances);
        let partitions = manager.partition_rows(&enriched, 2);
        assert_eq!(partitions.len(), 2);
        assert_eq!(partitions[0].len(), 2);
        assert_eq!(partitions[1].len(), 1);
    }
}