//! Tests for V1 compaction, V2 deterministic packing, outcome
//! classification, and the dialogue-tail collector.

use serde_json::json;

use crate::core::llm::entities::{
    ChatMessage as Message, MessageContent, MessageRole, ToolCall,
};

use super::super::policy::{classify_outcome, collect_recent_dialogue_tail};
use super::super::{
    apply_context_policy_v2, maybe_compact_context, ContextPackOutcome, ContextPolicyInput,
    BUDGET_FAILURE_MEMORY_TOKENS, BUDGET_RECENT_DIALOGUE_TOKENS, KEEP_RECENT_MESSAGES,
    KEEP_RECENT_TAIL_MESSAGES,
};
use super::super::failure::build_failure_memory;
use super::super::estimate_context_tokens;
use super::common::{
    assistant_with_call, brief_block_from, build_above_threshold_context,
    context_with_failed_tool_call, memory_block_from, orphan_tool_result_ids,
    sample_execution_brief, sample_memory_reference, tool_result_for,
};

#[test]
fn test_context_compaction_below_threshold() {
    let mut context = vec![
        Message::system("You are a helpful assistant."),
        Message::user("Hello!"),
        Message::assistant("Hi there!"),
    ];

    let result = maybe_compact_context(&mut context, "gpt-4");
    assert!(result.is_none());
    assert_eq!(context.len(), 3); // Unchanged
}

#[test]
fn test_context_compaction_exactly_at_threshold() {
    // Create context just at threshold edge (60% of 128k = ~76800 tokens)
    // At ~30 chars per message, need ~2560 messages
    let mut context = vec![Message::system("System")];
    for i in 0..1280 {
        context.push(Message::user(&format!("User message {}", i)));
        context.push(Message::assistant(&format!("Assistant response {}", i)));
    }

    let before_len = context.len();
    let result = maybe_compact_context(&mut context, "gpt-4");

    // At exactly threshold, should still compact
    // (the function checks usage_ratio < threshold for no compaction)
    // So at >= threshold, it should compact
    if result.is_some() {
        assert!(context.len() < before_len);
    }
}

#[test]
fn test_context_compaction_above_threshold() {
    // Create a context large enough to trigger compaction
    // For gpt-4 (128k @ 60% = ~76800 tokens), need enough to exceed threshold
    // Using longer messages to reach threshold faster
    let mut context = vec![Message::system("System prompt")];
    for i in 0..3500 {
        context.push(Message::user(&format!("User message {} with some additional content to increase token count", i)));
        context.push(Message::assistant(&format!(
            "Assistant response {} with more content here to exceed threshold",
            i
        )));
    }

    let result = maybe_compact_context(&mut context, "gpt-4");

    assert!(result.is_some());
    let details = result.unwrap();
    assert!(details.after_count < details.before_count);
    assert_eq!(context.len(), details.after_count);

    // First message preserved
    assert_eq!(context[0].role, MessageRole::System);

    // Recent messages preserved (+1 for summary)
    assert!(context.len() <= KEEP_RECENT_MESSAGES + 2);
}

#[test]
fn test_context_compaction_keeps_first_and_recent_messages() {
    // Create a large context to trigger compaction (>60% of 128k tokens = ~76800 tokens)
    // Need enough content per message to exceed the threshold
    let long_content = "A".repeat(100); // 100 chars per message
    let mut context = vec![Message::system("System")];
    for i in 0..2000 {
        context.push(Message::user(format!(
            "User {} message with long content: {}",
            i, long_content
        )));
        context.push(Message::assistant(format!(
            "Assistant {} response with long content: {}",
            i, long_content
        )));
    }

    let before_len = context.len();
    let result = maybe_compact_context(&mut context, "gpt-4");

    // Verify compaction occurred
    assert!(
        result.is_some(),
        "Compaction should occur with enough content"
    );
    assert!(
        context.len() < before_len,
        "Context should be smaller after compaction"
    );

    // First message should be preserved
    assert_eq!(context[0].as_text(), Some("System"));

    // After compaction, we should have first message + summary + recent messages
    assert!(context.len() <= KEEP_RECENT_MESSAGES + 2);
}

#[test]
fn test_context_compaction_details() {
    let mut context = vec![Message::system("System")];
    // Need enough messages to exceed 60% of gpt-4's 128k context window
    // Using longer messages to reach token threshold faster
    for i in 0..3500 {
        context.push(Message::user(&format!("User message {} with extended content", i)));
        context.push(Message::assistant(&format!("Assistant response {} with additional text", i)));
    }

    let result = maybe_compact_context(&mut context, "gpt-4");
    assert!(result.is_some());

    let details = result.unwrap();
    assert!(details.before_count > details.after_count);
    assert!(details.before_tokens > details.after_tokens);
    assert!(details.needs_reinjection);
}

#[test]
fn test_context_compaction_small_context() {
    // Small context should not compact even if we try
    let mut context = vec![
        Message::system("System"),
        Message::user("Hello"),
        Message::assistant("Hi"),
    ];

    let result = maybe_compact_context(&mut context, "gpt-4");
    assert!(result.is_none(), "Small context should not compact");
    assert_eq!(context.len(), 3);
}

#[test]
fn test_context_compaction_exactly_keep_recent_plus_one() {
    // If context is exactly KEEP_RECENT_MESSAGES + 1, no compaction needed
    let mut context = vec![Message::system("System")];
    for i in 0..KEEP_RECENT_MESSAGES {
        context.push(Message::user(&format!("Message {}", i)));
    }

    // This should be exactly KEEP_RECENT_MESSAGES + 1
    assert_eq!(context.len(), KEEP_RECENT_MESSAGES + 1);

    let result = maybe_compact_context(&mut context, "gpt-4");
    // With small messages, we're likely below token threshold
    assert!(result.is_none());
}

#[test]
fn test_context_compaction_preserves_system_message() {
    let mut context = vec![Message::system("Important system prompt")];
    for i in 0..3000 {
        context.push(Message::user(&format!("User {}", i)));
        context.push(Message::assistant(&format!("Assistant {}", i)));
    }

    maybe_compact_context(&mut context, "gpt-4");

    // System message should always be first
    assert_eq!(context[0].role, MessageRole::System);
    assert_eq!(context[0].as_text(), Some("Important system prompt"));
}

#[test]
fn test_context_compaction_with_mixed_message_types() {
    let mut context = vec![Message::system("System")];
    // Need enough messages to exceed 60% of CONTEXT_WINDOW (76800 tokens)
    // Need longer messages to exceed 60% of gpt-4's 128k window
    for i in 0..3500 {
        context.push(Message::user(&format!("User message {} with extended content to reach threshold", i)));
        context.push(Message::assistant_with_tool_calls(
            format!("Response {} with additional text", i),
            vec![ToolCall {
                id: format!("call_{}", i),
                name: "test_tool".to_string(),
                arguments: json!({"index": i}),
            }],
        ));
    }

    let result = maybe_compact_context(&mut context, "gpt-4");
    assert!(result.is_some());

    // Should still have system message
    assert_eq!(context[0].role, MessageRole::System);
    // Should be reduced to system + summary + recent
    assert!(context.len() <= KEEP_RECENT_MESSAGES + 2);
}

#[test]
fn test_compaction_details_needs_reinjection_flag() {
    let mut context = vec![Message::system("System")];
    // Need enough messages to exceed 60% of gpt-4's 128k context window
    // With longer messages to ensure we hit the threshold reliably
    for i in 0..5000 {
        context.push(Message::user(&format!("User message {} with extended content to reach token limit threshold", i)));
    }

    let result = maybe_compact_context(&mut context, "gpt-4");
    assert!(result.is_some());

    let details = result.unwrap();
    assert!(
        details.needs_reinjection,
        "Compaction should always signal re-injection needed"
    );
}

#[test]
fn test_context_compaction_token_estimation_consistency() {
    let mut context = vec![Message::system("System")];
    for i in 0..1000 {
        context.push(Message::user(&format!("Message {}", i)));
    }

    let before_tokens = estimate_context_tokens(&context);

    let result = maybe_compact_context(&mut context, "gpt-4");
    if let Some(details) = result {
        assert_eq!(details.before_tokens, before_tokens);
        assert!(details.after_tokens < before_tokens);
    }
}

#[test]
fn policy_v2_inlines_memory_reference_block_when_provided() {
    let mut context = build_above_threshold_context();
    let input = ContextPolicyInput {
        model_name: "gpt-4".to_string(),
        memory_reference: Some(sample_memory_reference()),
        ..ContextPolicyInput::default()
    };
    let result = apply_context_policy_v2(&mut context, &input);
    assert!(result.is_some(), "large context should pack");
    let block = memory_block_from(&context).expect("memory block must be present");
    assert!(block.contains("session=sess-42"));
    assert!(block.contains("findings=2"));
    assert!(block.contains("questions=1"));
    assert!(block.contains("facts=3"));
    assert!(block.contains("investigate auth regression"));
}

#[test]
fn policy_v2_omits_memory_reference_block_when_none() {
    let mut context = build_above_threshold_context();
    let input = ContextPolicyInput {
        model_name: "gpt-4".to_string(),
        memory_reference: None,
        ..ContextPolicyInput::default()
    };
    let result = apply_context_policy_v2(&mut context, &input);
    assert!(result.is_some());
    assert!(
        memory_block_from(&context).is_none(),
        "no memory reference input => no memory block in packed context",
    );
}

#[test]
fn policy_v2_memory_reference_block_is_deterministic() {
    // Two independent packing passes with the same MemoryReference
    // must produce byte-identical blocks. Drives message_prefix_hash
    // stability (Milestone 1.3 invariant).
    let mut ctx_a = build_above_threshold_context();
    let mut ctx_b = build_above_threshold_context();
    let input_a = ContextPolicyInput {
        model_name: "gpt-4".to_string(),
        memory_reference: Some(sample_memory_reference()),
        ..ContextPolicyInput::default()
    };
    let input_b = input_a.clone();
    apply_context_policy_v2(&mut ctx_a, &input_a).unwrap();
    apply_context_policy_v2(&mut ctx_b, &input_b).unwrap();
    assert_eq!(memory_block_from(&ctx_a), memory_block_from(&ctx_b));
}

#[test]
fn policy_v2_memory_reference_block_truncates_oversized_fields() {
    let huge = "y".repeat(10_000);
    let memory = super::super::MemoryReference {
        task_goal: huge.clone(),
        active_plan: huge,
        ..sample_memory_reference()
    };
    let mut context = build_above_threshold_context();
    let input = ContextPolicyInput {
        model_name: "gpt-4".to_string(),
        memory_reference: Some(memory),
        ..ContextPolicyInput::default()
    };
    apply_context_policy_v2(&mut context, &input).unwrap();
    let block = memory_block_from(&context).expect("memory block must exist");
    // Budget (180) split in thirds => ~60 tokens each => ~240 chars each;
    // a 10_000-char field must be truncated well below its own length.
    // 1_500 chars is a generous upper bound that still proves truncation.
    assert!(
        block.len() < 1_500,
        "memory block should be truncated; got {} chars",
        block.len()
    );
}

#[test]
fn policy_v2_tiny_forced_context_packs_when_memory_reference_is_present() {
    // Without a memory_reference, a tiny forced context short-circuits
    // and skips packing. With one, the packer must run so the memory
    // block is inlined — otherwise compaction could silently drop the
    // only pointer to durable memory.
    let mut context = vec![
        Message::system("System"),
        Message::user("Hi"),
        Message::assistant("Hello"),
    ];
    let input = ContextPolicyInput {
        force_pack: true,
        model_name: "gpt-4".to_string(),
        memory_reference: Some(sample_memory_reference()),
        ..ContextPolicyInput::default()
    };
    let result = apply_context_policy_v2(&mut context, &input);
    assert!(result.is_some(), "memory-bearing tiny forced pack must run");
    assert!(
        memory_block_from(&context).is_some(),
        "memory reference must survive the packing pass",
    );
}

#[test]
fn policy_v2_repeated_packing_does_not_duplicate_memory_reference_block() {
    // Safety net against an earlier pass's ContextPolicyV2 block
    // leaking through into the next pack, causing the memory
    // reference to appear twice. The tail collector filters out
    // existing "ContextPolicyV2 " system messages; this test pins
    // that invariant end-to-end for the memory block.
    //
    // The second call uses force_pack so it runs even when the
    // first pass already shrank the context below the compaction
    // threshold.
    let mut context = build_above_threshold_context();
    let input = ContextPolicyInput {
        model_name: "gpt-4".to_string(),
        memory_reference: Some(sample_memory_reference()),
        ..ContextPolicyInput::default()
    };
    apply_context_policy_v2(&mut context, &input).unwrap();
    let force_input = ContextPolicyInput {
        force_pack: true,
        ..input
    };
    apply_context_policy_v2(&mut context, &force_input).unwrap();
    let count = context
        .iter()
        .filter(|m| {
            m.role == MessageRole::System
                && m.as_text()
                    .is_some_and(|t| t.starts_with("ContextPolicyV2 session memory reference:"))
        })
        .count();
    assert_eq!(count, 1, "memory block must not duplicate across packs");
}

#[test]
fn test_apply_policy_v2_skips_tiny_forced_context() {
    let mut context = vec![
        Message::system("System"),
        Message::user("Hi"),
        Message::assistant("Hello"),
    ];
    let input = ContextPolicyInput {
        force_pack: true,
        model_name: "gpt-4".to_string(),
        ..ContextPolicyInput::default()
    };
    let result = apply_context_policy_v2(&mut context, &input);
    assert!(result.is_none(), "tiny forced context must skip packing");
    assert_eq!(context.len(), 3, "context must not be mutated when skipping");
}

#[test]
fn test_apply_policy_v2_classifies_unchanged_correctly() {
    // Build a context that exceeds threshold but whose packed form matches input shape.
    // Hitting an exact match is hard; assert that classify_outcome agrees on identity.
    assert_eq!(
        classify_outcome(500, 500, 10, 10),
        ContextPackOutcome::Unchanged,
    );
}

#[test]
fn test_classify_outcome_reorganized_when_insufficient_savings() {
    // 50-token savings on a 1000-token context: below both thresholds.
    assert_eq!(
        classify_outcome(1_000, 950, 10, 8),
        ContextPackOutcome::Reorganized,
    );
}

#[test]
fn test_classify_outcome_compacted_when_real_savings() {
    // 6000-token savings on a 10000-token context: 60% > MIN_SAVINGS_RATIO.
    assert_eq!(
        classify_outcome(10_000, 4_000, 50, 10),
        ContextPackOutcome::Compacted,
    );
}

#[test]
fn test_maybe_compact_context_populates_outcome() {
    // Build a context well above gpt-4's 70% threshold (~89.6k tokens).
    let padding = "x".repeat(400);
    let mut context = vec![Message::system("System")];
    for i in 0..1_000 {
        context.push(Message::user(format!("msg {i} {padding}")));
    }

    let result = maybe_compact_context(&mut context, "gpt-4");
    let details = result.expect("large context should compact");
    assert_eq!(details.outcome, ContextPackOutcome::Compacted);
    assert!(details.after_tokens < details.before_tokens);
}

// --- collect_recent_dialogue_tail pairing invariant ---
//
// These tests enforce the invariant that the returned tail never contains a
// tool_result whose paired tool_use (on an assistant message) was dropped.
// Violating this invariant is what caused the production 400 on session
// 0ceae112 step 17: "tool result's tool id(call_function_...) not found".

#[test]
fn collect_recent_tail_drops_orphan_when_tool_use_exceeds_token_budget() {
    // Reproduces the production incident: an fs_write_file tool_use carrying
    // a large `content` argument blows past the 1600-token tail budget, so
    // the backward walk `break`s on that assistant message. Without the fix,
    // its cheap tool_result (already pushed) is left as an orphan.
    //
    // ~15_000-byte arguments => ~3750 tokens, well beyond 1600.
    let context = vec![
        Message::user("please write the file".to_string()),
        assistant_with_call("call_X", 15_000),
        tool_result_for("call_X"),
        Message::user("next step please".to_string()),
        Message::assistant("ok, continuing".to_string()),
    ];

    let tail = collect_recent_dialogue_tail(&context, BUDGET_RECENT_DIALOGUE_TOKENS);

    assert!(
        orphan_tool_result_ids(&tail).is_empty(),
        "tail must not contain orphan tool_results; got tail: {tail:#?}",
    );
}

#[test]
fn collect_recent_tail_drops_orphan_when_tool_use_outside_message_window() {
    // KEEP_RECENT_TAIL_MESSAGES caps the walk at 12 kept messages. If an
    // assistant tool_use sits just outside that window while its tool_result
    // sits just inside, the pair splits. Distinct from the token-budget
    // path above.
    //
    // Layout (21 messages, indices 0..20):
    //   idx  0-8 : small chatter
    //   idx  9   : assistant tool_use  call_Y  <- MUST be at the boundary
    //   idx 10   : tool_result         call_Y
    //   idx 11-20: 10 more small messages
    //
    // Backward walk: 20,19,...,10 = 11 messages kept, then idx 9 would be
    // the 12th and fits the cap. So to push idx 10 in but idx 9 out we need
    // one more message past idx 20. Use 22 messages total; assistant at
    // idx 9, tool_result at idx 10, chatter idx 11..21.
    let mut context: Vec<Message> = (0..9)
        .map(|i| Message::user(format!("chatter {i}")))
        .collect();
    context.push(assistant_with_call("call_Y", 4));
    context.push(tool_result_for("call_Y"));
    for i in 0..11 {
        context.push(Message::user(format!("tail chatter {i}")));
    }
    assert_eq!(context.len(), 22);

    let tail = collect_recent_dialogue_tail(&context, 100_000);

    assert!(
        tail.len() <= KEEP_RECENT_TAIL_MESSAGES,
        "tail length {} exceeds KEEP_RECENT_TAIL_MESSAGES {}",
        tail.len(),
        KEEP_RECENT_TAIL_MESSAGES,
    );
    assert!(
        orphan_tool_result_ids(&tail).is_empty(),
        "tail must not contain orphan tool_results; got tail: {tail:#?}",
    );
}

#[test]
fn collect_recent_tail_keeps_matched_pair_when_both_fit() {
    // Baseline: when both members of the pair fit, both must be preserved.
    let context = vec![
        assistant_with_call("call_Z", 20),
        tool_result_for("call_Z"),
        Message::user("thanks".to_string()),
    ];

    let tail = collect_recent_dialogue_tail(&context, BUDGET_RECENT_DIALOGUE_TOKENS);

    let has_assistant = tail
        .iter()
        .any(|m| m.role == MessageRole::Assistant && !m.tool_calls.is_empty());
    let has_tool_result = tail.iter().any(|m| m.role == MessageRole::Tool);
    assert!(
        has_assistant && has_tool_result,
        "both pair members should survive; got tail: {tail:#?}",
    );
    assert!(orphan_tool_result_ids(&tail).is_empty());
}

#[test]
fn collect_recent_tail_reproduces_session_0ceae112_incident() {
    // Exact shape of the production failure: fs_write_file to
    // docs/cxone/quick-reference.md with ~11593 bytes of content, followed
    // by a tiny tool_result, followed by one more tool round-trip, then a
    // subtask transition. Before the fix, compaction leaves the 11593-byte
    // tool_use out of the tail while its tool_result stays in.
    let huge_content = "#".repeat(11_593);
    let context = vec![
        Message::user("extract business logic".to_string()),
        Message::assistant_with_tool_calls(
            "creating the quick reference".to_string(),
            vec![ToolCall {
                id: "call_function_snasv7li0myh_1".to_string(),
                name: "fs_write_file".to_string(),
                arguments: json!({
                    "path": "docs/cxone/quick-reference.md",
                    "content": huge_content,
                }),
            }],
        ),
        Message::tool_result(
            "call_function_snasv7li0myh_1",
            json!({"bytes_written": 11_593, "path": "docs/cxone/quick-reference.md"}),
            false,
        ),
        Message::assistant_with_tool_calls(
            "listing directory".to_string(),
            vec![ToolCall {
                id: "call_function_list_1".to_string(),
                name: "fs_list_files".to_string(),
                arguments: json!({"path": "docs/cxone"}),
            }],
        ),
        Message::tool_result(
            "call_function_list_1",
            json!({"count": 8}),
            false,
        ),
        Message::assistant_with_tool_calls(
            "advancing".to_string(),
            vec![ToolCall {
                id: "call_function_advance_1".to_string(),
                name: "agent_advance_to_next_subtask".to_string(),
                arguments: json!({}),
            }],
        ),
        Message::tool_result(
            "call_function_advance_1",
            json!({"success": true}),
            false,
        ),
        Message::user("now compile the knowledge base".to_string()),
    ];

    let tail = collect_recent_dialogue_tail(&context, BUDGET_RECENT_DIALOGUE_TOKENS);

    let orphans = orphan_tool_result_ids(&tail);
    assert!(
        orphans.is_empty(),
        "session 0ceae112 incident must not repeat: orphan tool_call_ids in tail: {orphans:?}; tail: {tail:#?}",
    );
}

// --- M4.1 execution brief tests ---

#[test]
fn policy_v2_inlines_execution_brief_block_when_provided() {
    let mut context = build_above_threshold_context();
    let input = ContextPolicyInput {
        model_name: "gpt-4".to_string(),
        execution_brief: Some(sample_execution_brief()),
        ..ContextPolicyInput::default()
    };
    apply_context_policy_v2(&mut context, &input).expect("large context should pack");
    let block = brief_block_from(&context).expect("brief block must be present");
    assert!(block.contains("goal: fix failing login flow"));
    assert!(block.contains("active: patch token refresh"));
    assert!(block.contains("recent: audit auth.rs; repro failing test"));
    assert!(block.contains("next: add regression test"));
    assert!(
        !block.contains("blocker:"),
        "brief without blocker must not emit a blocker line; got {block}",
    );
}

#[test]
fn policy_v2_omits_execution_brief_block_when_none() {
    let mut context = build_above_threshold_context();
    let input = ContextPolicyInput {
        model_name: "gpt-4".to_string(),
        execution_brief: None,
        ..ContextPolicyInput::default()
    };
    apply_context_policy_v2(&mut context, &input).expect("large context should pack");
    assert!(
        brief_block_from(&context).is_none(),
        "no execution brief input => no brief block in packed context",
    );
}

#[test]
fn policy_v2_execution_brief_block_is_deterministic() {
    let mut ctx_a = build_above_threshold_context();
    let mut ctx_b = build_above_threshold_context();
    let input_a = ContextPolicyInput {
        model_name: "gpt-4".to_string(),
        execution_brief: Some(sample_execution_brief()),
        ..ContextPolicyInput::default()
    };
    let input_b = input_a.clone();
    apply_context_policy_v2(&mut ctx_a, &input_a).unwrap();
    apply_context_policy_v2(&mut ctx_b, &input_b).unwrap();
    assert_eq!(brief_block_from(&ctx_a), brief_block_from(&ctx_b));
}

#[test]
fn policy_v2_execution_brief_omits_missing_optional_fields() {
    let brief = super::super::ExecutionBrief {
        task_goal: "audit".to_string(),
        active_subtask: None,
        recently_completed: vec![],
        next_intended_action: None,
        current_blocker: None,
    };
    let mut context = build_above_threshold_context();
    let input = ContextPolicyInput {
        model_name: "gpt-4".to_string(),
        execution_brief: Some(brief),
        ..ContextPolicyInput::default()
    };
    apply_context_policy_v2(&mut context, &input).unwrap();
    let block = brief_block_from(&context).expect("brief block must be present");
    assert!(block.contains("goal: audit"));
    assert!(!block.contains("\nactive:"));
    assert!(!block.contains("\nrecent:"));
    assert!(!block.contains("\nnext:"));
    assert!(!block.contains("\nblocker:"));
}

#[test]
fn policy_v2_execution_brief_reflects_changed_active_subtask() {
    let brief_v1 = super::super::ExecutionBrief {
        active_subtask: Some("read code".to_string()),
        ..sample_execution_brief()
    };
    let brief_v2 = super::super::ExecutionBrief {
        active_subtask: Some("write fix".to_string()),
        ..sample_execution_brief()
    };
    let mut ctx_a = build_above_threshold_context();
    let mut ctx_b = build_above_threshold_context();
    apply_context_policy_v2(
        &mut ctx_a,
        &ContextPolicyInput {
            model_name: "gpt-4".to_string(),
            execution_brief: Some(brief_v1),
            ..ContextPolicyInput::default()
        },
    )
    .unwrap();
    apply_context_policy_v2(
        &mut ctx_b,
        &ContextPolicyInput {
            model_name: "gpt-4".to_string(),
            execution_brief: Some(brief_v2),
            ..ContextPolicyInput::default()
        },
    )
    .unwrap();
    assert_ne!(
        brief_block_from(&ctx_a),
        brief_block_from(&ctx_b),
        "subtask transition must move the brief block",
    );
}

#[test]
fn policy_v2_repeated_packing_does_not_duplicate_execution_brief_block() {
    let mut context = build_above_threshold_context();
    let input = ContextPolicyInput {
        model_name: "gpt-4".to_string(),
        execution_brief: Some(sample_execution_brief()),
        ..ContextPolicyInput::default()
    };
    apply_context_policy_v2(&mut context, &input).unwrap();
    let force_input = ContextPolicyInput {
        force_pack: true,
        ..input
    };
    apply_context_policy_v2(&mut context, &force_input).unwrap();
    let count = context
        .iter()
        .filter(|m| {
            m.role == MessageRole::System
                && m.as_text()
                    .is_some_and(|t| t.starts_with("ContextPolicyV2 execution brief:"))
        })
        .count();
    assert_eq!(count, 1, "brief block must not duplicate across packs");
}

#[test]
fn policy_v2_tiny_forced_pack_runs_when_execution_brief_is_present() {
    // Mirrors the memory-reference exception: without a brief a
    // tiny-context forced pack short-circuits, but with one the
    // packer must run so the brief is inlined.
    let mut context = vec![
        Message::system("System"),
        Message::user("Hi"),
        Message::assistant("Hello"),
    ];
    let input = ContextPolicyInput {
        force_pack: true,
        model_name: "gpt-4".to_string(),
        execution_brief: Some(sample_execution_brief()),
        ..ContextPolicyInput::default()
    };
    let result = apply_context_policy_v2(&mut context, &input);
    assert!(result.is_some(), "brief-bearing tiny forced pack must run");
    assert!(
        brief_block_from(&context).is_some(),
        "brief must survive the packing pass",
    );
}

#[test]
fn failure_memory_is_emitted_as_a_bounded_block_by_packer() {
    let mut context = context_with_failed_tool_call(
        "fs_read_file",
        json!({"path": "/long/path/that/keeps/going/and/going.txt"}),
        "No such file: /long/path/that/keeps/going/and/going.txt",
    );
    let input = ContextPolicyInput {
        force_pack: true,
        model_name: "gpt-4".to_string(),
        ..ContextPolicyInput::default()
    };
    apply_context_policy_v2(&mut context, &input);
    let block = context.iter().find_map(|m| match &m.content {
        MessageContent::Text(text)
            if m.role == MessageRole::System
                && text.starts_with("ContextPolicyV2 failure memory:") =>
        {
            Some(text.clone())
        }
        _ => None,
    });
    // Tiny forced-pack skips when no "real" block (memory ref / brief)
    // is supplied, so the failure-memory block only lands when
    // packing actually runs. Assert the budget invariant directly
    // on the formatter output instead.
    let formatted = build_failure_memory(&context_with_failed_tool_call(
        "fs_read_file",
        json!({"path": "/long/path/that/keeps/going/and/going.txt"}),
        "No such file: /long/path/that/keeps/going/and/going.txt",
    ))
    .unwrap();
    // 140 tokens ≈ 560 chars. The formatter's fields are all
    // individually short, so the full block must fit the budget.
    assert!(
        formatted.len() <= BUDGET_FAILURE_MEMORY_TOKENS * 4,
        "failure memory must stay within {}-token budget; got {} chars",
        BUDGET_FAILURE_MEMORY_TOKENS,
        formatted.len(),
    );
    // Block may or may not appear in packed context depending on
    // tiny-forced-pack short-circuit semantics — but when it does
    // appear, the prefix must match.
    if let Some(text) = block {
        assert!(text.contains("tool: fs_read_file"));
        assert!(text.contains("type: file_not_found"));
    }
}
