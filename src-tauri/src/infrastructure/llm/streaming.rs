//! LLM streaming module.
//!
//! Provides streaming support for LLM responses with chunk batching
//! to reduce IPC overhead.

pub mod batcher;

pub use batcher::{BatchingConfig, ChunkBatcher};

use async_stream::stream;
use futures::Stream;
use std::pin::Pin;

use crate::core::llm::entities::StreamChunk;
use crate::core::llm::services::LlmEvent;
use crate::infrastructure::llm::LlmError;

/// Creates a streaming helper for converting chunks to events.
///
/// This function batches chunks to reduce IPC overhead:
/// - Chunks are accumulated and sent based on the batching config
/// - This reduces ~150 IPC calls to ~20-30 per response
#[must_use]
pub fn stream_to_events(
    request_id: String,
    model: String,
    chunks: Pin<Box<dyn Stream<Item = Result<StreamChunk, LlmError>> + Send>>,
) -> Pin<Box<dyn Stream<Item = LlmEvent> + Send>> {
    stream_to_events_with_thread(request_id, model, chunks, BatchingConfig::default(), None)
}

/// Creates a streaming helper with custom batching configuration.
#[must_use]
pub fn stream_to_events_with_config(
    request_id: String,
    model: String,
    chunks: Pin<Box<dyn Stream<Item = Result<StreamChunk, LlmError>> + Send>>,
    config: BatchingConfig,
) -> Pin<Box<dyn Stream<Item = LlmEvent> + Send>> {
    stream_to_events_with_thread(request_id, model, chunks, config, None)
}

/// Creates a streaming helper with custom batching configuration and thread ID.
#[must_use]
pub fn stream_to_events_with_thread(
    request_id: String,
    model: String,
    chunks: Pin<Box<dyn Stream<Item = Result<StreamChunk, LlmError>> + Send>>,
    config: BatchingConfig,
    thread_id: Option<String>,
) -> Pin<Box<dyn Stream<Item = LlmEvent> + Send>> {
    Box::pin(stream! {
        yield LlmEvent::Started {
            request_id: request_id.clone(),
            model: model.clone(),
            thread_id: thread_id.clone(),
        };

        let mut batcher = ChunkBatcher::new(config);

        #[allow(clippy::expect_used)]
        let mut chunks = std::pin::pin!(chunks);

        while let Some(chunk_result) = futures::StreamExt::next(&mut chunks).await {
            match chunk_result {
                Ok(chunk) => {
                    if let Some(events) = batcher.process_chunk(&chunk, &request_id, thread_id.clone()) {
                        for event in events {
                            yield event;
                        }
                    }
                }
                Err(e) => {
                    // Flush any remaining content before error
                    if let Some(flush_event) = batcher.flush(&request_id, thread_id.clone()) {
                        yield flush_event;
                    }
                    yield LlmEvent::Error {
                        request_id: request_id.clone(),
                        error: e.to_string(),
                        retryable: e.is_retryable(),
                        thread_id: thread_id.clone(),
                    };
                    return;
                }
            }
        }

        // Flush any remaining buffered content
        if let Some(_final_event) = batcher.finish(&request_id, thread_id.clone()) {
            // Event yielded
        }

        // Get the final values before moving batcher
        let _full_content = batcher.full_content();
        let finish_reason = batcher.finish_reason();
        let full_content_owned = batcher.into_full_content();

        yield LlmEvent::Complete {
            request_id,
            content: full_content_owned,
            usage: None, // Will be set by streaming module
            finish_reason,
            thread_id,
        };
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use std::time::Duration;

    fn create_test_chunk(id: &str, content: &str, is_final: bool) -> StreamChunk {
        StreamChunk {
            id: id.to_string(),
            model: "test-model".to_string(),
            delta: Some(StreamDelta {
                role: None,
                content: Some(content.to_string()),
                reasoning_content: None,
            }),
            usage: None,
            finish_reason: if is_final {
                Some("stop".to_string())
            } else {
                None
            },
            is_final,
        }
    }

    #[tokio::test]
    async fn test_stream_to_events_basic() {
        let chunks = futures::stream::iter(vec![
            Ok(create_test_chunk("resp-1", "Hello", false)),
            Ok(create_test_chunk("resp-1", " world", true)),
        ]);

        let events: Vec<LlmEvent> =
            stream_to_events("req-1".to_string(), "gpt-4".to_string(), Box::pin(chunks))
                .collect()
                .await;

        // With default batching (min_flush_length: 20), small chunks are batched
        // So we expect: Started, Complete (no intermediate Chunk events)
        assert_eq!(events.len(), 2);
        assert!(matches!(
            &events[0],
            LlmEvent::Started { request_id, .. } if request_id == "req-1"
        ));
        assert!(matches!(&events[1], LlmEvent::Complete { .. }));
    }

    #[tokio::test]
    async fn test_stream_to_events_content_accumulation() {
        let chunks = futures::stream::iter(vec![
            Ok(create_test_chunk("resp-1", "Hello", false)),
            Ok(create_test_chunk("resp-1", " world", true)),
        ]);

        let events: Vec<LlmEvent> =
            stream_to_events("req-1".to_string(), "gpt-4".to_string(), Box::pin(chunks))
                .collect()
                .await;

        let complete_event = events.iter().find_map(|e| match e {
            LlmEvent::Complete { content, .. } => Some(content.clone()),
            _ => None,
        });

        assert_eq!(complete_event.as_deref(), Some("Hello world"));
    }

    #[tokio::test]
    async fn test_stream_to_events_error_handling() {
        let chunks = futures::stream::iter(vec![
            Ok(create_test_chunk("resp-1", "partial", false)),
            Err(LlmError::rate_limit(Some(5))),
        ]);

        let events: Vec<LlmEvent> =
            stream_to_events("req-2".to_string(), "gpt-4".to_string(), Box::pin(chunks))
                .collect()
                .await;

        let has_error = events.iter().any(|e| matches!(e, LlmEvent::Error { .. }));
        assert!(has_error);
    }
}
