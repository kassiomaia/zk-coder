use crate::core::chat::entities::ChatMessage;
use crate::core::llm::entities::MessageRole as CoreMessageRole;

pub use super::helpers::*;

mod tests {
    use super::*;
    use crate::core::chat::entities::MessageFeedback;

    #[test]
    fn test_convert_messages() {
        let messages = vec![
            ChatMessage::user("conv-1", "Hello"),
            ChatMessage::assistant("conv-1", "Hi there!"),
        ];

        let llm_messages = convert_messages(&messages);

        assert_eq!(llm_messages.len(), 2);
        assert_eq!(llm_messages[0].role, CoreMessageRole::User);
        assert_eq!(llm_messages[1].role, CoreMessageRole::Assistant);
    }

    #[test]
    fn test_build_feedback_summary_empty() {
        let feedback: Vec<(ChatMessage, MessageFeedback)> = vec![];
        let summary = build_feedback_summary(&feedback);
        assert!(summary.is_none());
    }

    #[test]
    fn test_build_feedback_summary_with_likes() {
        let msg1 = ChatMessage::assistant("conv-1", "Response 1");
        let msg2 = ChatMessage::assistant("conv-1", "Response 2");

        let feedback = vec![
            (msg1, MessageFeedback::Like),
            (msg2, MessageFeedback::Dislike),
        ];

        let summary = build_feedback_summary(&feedback);
        assert!(summary.is_some());

        let summary = summary.unwrap();
        assert!(summary.contains("Feedback on previous messages:"));
        assert!(summary.contains("LIKED"));
        assert!(summary.contains("DISLIKED"));
        assert!(summary.contains("[ID:"));
    }

    #[test]
    fn test_build_system_prompt_with_feedback() {
        let original = "You are a helpful assistant.";
        let feedback_summary =
            Some("Feedback on previous messages:\n- [ID:abc12345] LIKED by user");

        let combined = build_system_prompt_with_feedback(original, feedback_summary);

        assert!(combined.contains("You are a helpful assistant."));
        assert!(combined.contains("including stable IDs for reference"));
        assert!(combined.contains("Feedback on previous messages:"));
    }

    #[test]
    fn test_build_system_prompt_without_feedback() {
        let original = "You are a helpful assistant.";
        let combined = build_system_prompt_with_feedback(original, None);

        // Should return original when no feedback
        assert_eq!(combined, original);
    }

    #[test]
    fn test_build_system_prompt_with_feedback_and_empty_original() {
        let feedback_summary =
            Some("Feedback on previous messages:\n- [ID:abc12345] DISLIKED by user");

        let combined = build_system_prompt_with_feedback("", feedback_summary);

        assert!(combined.starts_with(FEEDBACK_SYSTEM_PROMPT));
        assert!(combined.contains("Feedback on previous messages:"));
        assert!(combined.contains("Now continue the conversation below:"));
    }

    #[test]
    fn test_build_feedback_summary_uses_stable_short_ids() {
        let msg = ChatMessage::assistant("conv-1", "Response");
        let full_id = msg.id.clone();

        let summary = build_feedback_summary(&[(msg, MessageFeedback::Like)])
            .expect("summary should exist for one feedback item");

        let short_id = &full_id[..8];
        assert!(summary.contains(&format!("[ID:{}]", short_id)));
        assert!(!summary.contains(&format!("[ID:{}]", full_id)));
    }
}
