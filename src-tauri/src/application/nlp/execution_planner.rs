//! Execution planning for multi-intent queries.
//!
//! This module creates execution plans for multiple intents,
//! determining order and parallelism.

use std::collections::HashSet;

use crate::core::nlp::multi_intent::{IntentRelationship, MultiIntentResult};

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
                        Some(
                            IntentRelationship::Sequential
                                | IntentRelationship::Dependent
                                | IntentRelationship::Exclusive
                        )
                    ) {
                        // Start new level for sequential, dependent, or exclusive relationships
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
    use crate::core::nlp::intent::Intent;
    use crate::core::nlp::multi_intent::{DetectedIntent, IntentRelationship, TextSpan};

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

    fn create_parallel_result() -> MultiIntentResult {
        MultiIntentResult {
            intents: vec![
                DetectedIntent::new(Intent::FormatCode, TextSpan::new(0, 6, "format"), 0.9),
                DetectedIntent::new(Intent::LintCode, TextSpan::new(7, 11, "lint"), 0.9),
            ],
            relationships: vec![IntentRelationship::Parallel],
            execution_order: vec![0, 1],
        }
    }

    fn create_dependent_result() -> MultiIntentResult {
        MultiIntentResult {
            intents: vec![
                DetectedIntent::new(Intent::FindBugs, TextSpan::new(0, 9, "find bugs"), 0.9),
                DetectedIntent::new(
                    Intent::GenerateTest,
                    TextSpan::new(10, 20, "write tests"),
                    0.85,
                ),
            ],
            relationships: vec![IntentRelationship::Dependent],
            execution_order: vec![0, 1],
        }
    }

    fn create_complex_result() -> MultiIntentResult {
        MultiIntentResult {
            intents: vec![
                DetectedIntent::new(Intent::FindBugs, TextSpan::new(0, 9, "find bugs"), 0.9),
                DetectedIntent::new(
                    Intent::GenerateTest,
                    TextSpan::new(10, 20, "write tests"),
                    0.85,
                ),
                DetectedIntent::new(
                    Intent::DocumentCode,
                    TextSpan::new(21, 37, "add documentation"),
                    0.8,
                ),
            ],
            relationships: vec![IntentRelationship::Sequential, IntentRelationship::Parallel],
            execution_order: vec![0, 1, 2],
        }
    }

    fn create_result_with_invalid_execution_order() -> MultiIntentResult {
        MultiIntentResult {
            intents: vec![DetectedIntent::new(
                Intent::ExplainCode,
                TextSpan::new(0, 7, "explain"),
                0.9,
            )],
            relationships: vec![],
            execution_order: vec![0, 99], // Invalid index
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

    #[test]
    fn test_parallel_execution_single_level() {
        let result = create_parallel_result();
        let plan = ExecutionPlanner::create(&result);

        // Parallel intents should be in the same level
        assert_eq!(plan.level_count(), 1);
        assert_eq!(plan.intents_at_level(0).len(), 2);
        assert!(plan.intents_at_level(0).contains(&0));
        assert!(plan.intents_at_level(0).contains(&1));
    }

    #[test]
    fn test_sequential_execution_multiple_levels() {
        let result = create_multi_result();
        let plan = ExecutionPlanner::create(&result);

        // Sequential intents should be in different levels
        assert!(plan.level_count() >= 2);
        // First level should have the first intent
        assert_eq!(plan.intents_at_level(0), &[0]);
        // Second level should have the second intent
        assert_eq!(plan.intents_at_level(1), &[1]);
    }

    #[test]
    fn test_dependent_execution_multiple_levels() {
        let result = create_dependent_result();
        let plan = ExecutionPlanner::create(&result);

        // Dependent intents should be in different levels
        assert!(plan.level_count() >= 2);
        assert_eq!(plan.intents_at_level(0), &[0]);
        assert_eq!(plan.intents_at_level(1), &[1]);
    }

    #[test]
    fn test_complex_mixed_relationships() {
        let result = create_complex_result();
        let plan = ExecutionPlanner::create(&result);

        // Should handle mixed relationships
        assert!(plan.level_count() >= 2);
    }

    #[test]
    fn test_invalid_execution_order_indices() {
        let result = create_result_with_invalid_execution_order();
        let plan = ExecutionPlanner::create(&result);

        // Should handle invalid indices gracefully
        assert_eq!(plan.level_count(), 1);
    }

    #[test]
    fn test_execution_order_with_no_relationships() {
        let result = MultiIntentResult {
            intents: vec![
                DetectedIntent::new(Intent::GenerateCode, TextSpan::new(0, 6, "create"), 0.9),
                DetectedIntent::new(Intent::ExplainCode, TextSpan::new(7, 14, "explain"), 0.8),
            ],
            relationships: vec![],
            execution_order: vec![0, 1],
        };

        let plan = ExecutionPlanner::create(&result);

        // With no relationships, all intents should be in the first level
        assert_eq!(plan.level_count(), 1);
        assert_eq!(plan.intents_at_level(0).len(), 2);
    }

    #[test]
    fn test_intents_at_level_out_of_bounds() {
        let result = create_simple_result();
        let plan = ExecutionPlanner::create(&result);

        // Out of bounds should return empty
        assert_eq!(plan.intents_at_level(99), &[] as &[usize]);
        assert_eq!(plan.intents_at_level(1), &[] as &[usize]);
    }

    #[test]
    fn test_execution_with_three_sequential_intents() {
        let result = MultiIntentResult {
            intents: vec![
                DetectedIntent::new(Intent::FindBugs, TextSpan::new(0, 9, "find bugs"), 0.9),
                DetectedIntent::new(
                    Intent::GenerateTest,
                    TextSpan::new(10, 20, "write tests"),
                    0.85,
                ),
                DetectedIntent::new(Intent::DocumentCode, TextSpan::new(21, 37, "add docs"), 0.8),
            ],
            relationships: vec![
                IntentRelationship::Sequential,
                IntentRelationship::Sequential,
            ],
            execution_order: vec![0, 1, 2],
        };

        let plan = ExecutionPlanner::create(&result);

        // Each should be in its own level
        assert_eq!(plan.level_count(), 3);
        assert_eq!(plan.intents_at_level(0), &[0]);
        assert_eq!(plan.intents_at_level(1), &[1]);
        assert_eq!(plan.intents_at_level(2), &[2]);
    }

    #[test]
    fn test_execution_with_exclusive_relationships() {
        let result = MultiIntentResult {
            intents: vec![
                DetectedIntent::new(Intent::RefactorCode, TextSpan::new(0, 8, "refactor"), 0.8),
                DetectedIntent::new(Intent::OptimizeCode, TextSpan::new(9, 18, "optimize"), 0.8),
            ],
            relationships: vec![IntentRelationship::Exclusive],
            execution_order: vec![0, 1],
        };

        let plan = ExecutionPlanner::create(&result);

        // Exclusive should be treated like sequential
        assert!(plan.level_count() >= 2);
    }

    #[test]
    fn test_has_multiple_levels() {
        let single = create_simple_result();
        let single_plan = ExecutionPlanner::create(&single);
        assert!(!single_plan.has_multiple_levels());

        let multi = create_multi_result();
        let multi_plan = ExecutionPlanner::create(&multi);
        assert!(multi_plan.has_multiple_levels());
    }

    #[test]
    fn test_empty_plan_level_count() {
        let result = MultiIntentResult {
            intents: vec![],
            relationships: vec![],
            execution_order: vec![],
        };

        let plan = ExecutionPlanner::create(&result);
        assert_eq!(plan.level_count(), 0);
    }

    #[test]
    fn test_plan_levels_contain_valid_indices() {
        let result = create_complex_result();
        let plan = ExecutionPlanner::create(&result);

        for level_idx in 0..plan.level_count() {
            for &intent_idx in plan.intents_at_level(level_idx) {
                assert!(
                    intent_idx < result.intents.len(),
                    "Invalid intent index {intent_idx} at level {level_idx}"
                );
            }
        }
    }

    #[test]
    fn test_no_execution_order_uses_relationships() {
        let result = MultiIntentResult {
            intents: vec![
                DetectedIntent::new(Intent::FindBugs, TextSpan::new(0, 9, "find bugs"), 0.9),
                DetectedIntent::new(
                    Intent::GenerateTest,
                    TextSpan::new(10, 20, "write tests"),
                    0.85,
                ),
            ],
            relationships: vec![IntentRelationship::Sequential],
            execution_order: vec![], // Empty execution order
        };

        let plan = ExecutionPlanner::create(&result);

        // Should still create a plan based on relationships
        assert!(plan.level_count() >= 1);
    }

    #[test]
    fn test_plan_with_circular_dependency_handling() {
        // Create a situation that might cause circular dependency issues
        let result = MultiIntentResult {
            intents: vec![
                DetectedIntent::new(Intent::FindBugs, TextSpan::new(0, 9, "find bugs"), 0.9),
                DetectedIntent::new(
                    Intent::GenerateTest,
                    TextSpan::new(10, 20, "write tests"),
                    0.85,
                ),
            ],
            relationships: vec![IntentRelationship::Dependent],
            execution_order: vec![],
        };

        let plan = ExecutionPlanner::create(&result);

        // Should handle gracefully and create a plan
        assert!(plan.level_count() >= 1);
    }

    #[test]
    fn test_plan_levels_are_unique() {
        let result = create_complex_result();
        let plan = ExecutionPlanner::create(&result);

        let mut all_indices = Vec::new();
        for level_idx in 0..plan.level_count() {
            for &intent_idx in plan.intents_at_level(level_idx) {
                all_indices.push(intent_idx);
            }
        }

        // Check no duplicates when using execution_order
        let mut unique_indices = all_indices.clone();
        unique_indices.sort();
        unique_indices.dedup();

        if !result.execution_order.is_empty() {
            assert_eq!(
                all_indices.len(),
                unique_indices.len(),
                "Duplicate intent indices found"
            );
        }
    }

    #[test]
    fn test_execution_plan_multiple_parallel_levels() {
        let result = MultiIntentResult {
            intents: vec![
                DetectedIntent::new(Intent::FindBugs, TextSpan::new(0, 9, "find bugs"), 0.9),
                DetectedIntent::new(
                    Intent::GenerateTest,
                    TextSpan::new(10, 20, "write tests"),
                    0.85,
                ),
                DetectedIntent::new(Intent::FormatCode, TextSpan::new(21, 28, "format"), 0.9),
                DetectedIntent::new(Intent::LintCode, TextSpan::new(29, 33, "lint"), 0.9),
            ],
            relationships: vec![
                IntentRelationship::Sequential,
                IntentRelationship::Parallel,
                IntentRelationship::Parallel,
            ],
            execution_order: vec![0, 1, 2, 3],
        };

        let plan = ExecutionPlanner::create(&result);

        // First level: find bugs
        // Second level: write tests (sequential)
        // Third level: format and lint (parallel)
        assert!(plan.level_count() >= 2);

        // Check that format and lint are in the same level if parallel
        let last_level = plan.level_count() - 1;
        let last_level_intents = plan.intents_at_level(last_level);
        if last_level_intents.len() > 1 {
            assert!(last_level_intents.contains(&2));
            assert!(last_level_intents.contains(&3));
        }
    }
}
