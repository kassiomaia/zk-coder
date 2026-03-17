//! Execution planning for multi-intent queries.
//!
//! This module creates execution plans for multiple intents,
//! determining order and parallelism.

use std::collections::HashSet;

use super::multi_intent::{IntentRelationship, MultiIntentResult};

/// Execution plan for multiple intents.
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    /// Levels of execution (intents at same level can run in parallel)
    pub levels: Vec<Vec<usize>>,
}

impl ExecutionPlan {
    /// Returns the total number of levels.
    #[must_use]
    pub fn level_count(&self) -> usize {
        self.levels.len()
    }

    /// Returns intents at a specific level.
    #[must_use]
    pub fn intents_at_level(&self, level: usize) -> &[usize] {
        self.levels.get(level).map_or(&[], |v| v)
    }

    /// Returns true if there are multiple levels.
    #[must_use]
    pub fn has_multiple_levels(&self) -> bool {
        self.levels.len() > 1
    }
}

/// Creates execution plans from multi-intent results.
pub struct ExecutionPlanner;

impl ExecutionPlanner {
    /// Creates an execution plan from a multi-intent result.
    #[must_use]
    pub fn create(result: &MultiIntentResult) -> ExecutionPlan {
        let mut levels = Vec::new();
        let mut completed = HashSet::new();

        // If there's already an execution order, use it
        if !result.execution_order.is_empty() {
            // Group by relationships
            let mut current_level = Vec::new();

            for (i, &idx) in result.execution_order.iter().enumerate() {
                if idx >= result.intents.len() {
                    continue;
                }

                // Check relationship with previous
                if i > 0 {
                    let rel = result.relationships.get(i - 1);
                    if matches!(
                        rel,
                        Some(IntentRelationship::Sequential | IntentRelationship::Dependent)
                    ) {
                        // Start new level
                        if !current_level.is_empty() {
                            levels.push(current_level.clone());
                            current_level.clear();
                        }
                    }
                }
                current_level.push(idx);
                completed.insert(idx);
            }

            if !current_level.is_empty() {
                levels.push(current_level);
            }
        } else {
            // No explicit order - use relationships
            while completed.len() < result.intents.len() {
                let mut level = Vec::new();

                for (i, _) in result.intents.iter().enumerate() {
                    if completed.contains(&i) {
                        continue;
                    }

                    if Self::can_execute(result, i, &completed) {
                        level.push(i);
                        completed.insert(i);
                    }
                }

                if level.is_empty() {
                    // Circular dependency or error - add remaining
                    for i in 0..result.intents.len() {
                        if !completed.contains(&i) {
                            level.push(i);
                            completed.insert(i);
                        }
                    }
                }

                if !level.is_empty() {
                    levels.push(level);
                }
            }
        }

        ExecutionPlan { levels }
    }

    fn can_execute(result: &MultiIntentResult, index: usize, completed: &HashSet<usize>) -> bool {
        // Check if all dependencies are satisfied
        for (i, rel) in result.relationships.iter().enumerate() {
            if matches!(
                rel,
                IntentRelationship::Dependent | IntentRelationship::Sequential
            ) {
                // If this intent depends on a previous one
                if i < index && !completed.contains(&i) {
                    return false;
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nlp::intent::Intent;
    use crate::nlp::multi_intent::{DetectedIntent, TextSpan};

    fn create_simple_result() -> MultiIntentResult {
        let detected = DetectedIntent::new(Intent::GenerateCode, TextSpan::full("test"), 0.9);
        MultiIntentResult::single(detected)
    }

    fn create_multi_result() -> MultiIntentResult {
        MultiIntentResult {
            intents: vec![
                DetectedIntent::new(Intent::FindBugs, TextSpan::new(0, 10, "find bugs"), 0.9),
                DetectedIntent::new(
                    Intent::GenerateTest,
                    TextSpan::new(15, 25, "write tests"),
                    0.85,
                ),
            ],
            relationships: vec![IntentRelationship::Sequential],
            execution_order: vec![0, 1],
        }
    }

    #[test]
    fn test_single_intent_plan() {
        let result = create_simple_result();
        let plan = ExecutionPlanner::create(&result);

        assert_eq!(plan.level_count(), 1);
        assert_eq!(plan.intents_at_level(0), &[0]);
        assert!(!plan.has_multiple_levels());
    }

    #[test]
    fn test_multi_intent_plan() {
        let result = create_multi_result();
        let plan = ExecutionPlanner::create(&result);

        assert!(plan.level_count() >= 1);
    }

    #[test]
    fn test_empty_result() {
        let result = MultiIntentResult {
            intents: vec![],
            relationships: vec![],
            execution_order: vec![],
        };
        let plan = ExecutionPlanner::create(&result);

        assert_eq!(plan.level_count(), 0);
    }
}
