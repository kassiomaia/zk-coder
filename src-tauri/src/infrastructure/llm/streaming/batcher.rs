//! Chunk batching for LLM streaming.
//!
//! Batches small chunks together to reduce IPC overhead while
//! maintaining responsive streaming.

use crate::core::llm::entities::{StreamChunk, StreamDelta, UsageStats};
use crate::core::llm::services::LlmEvent;

/// Default configuration for chunk batching.
#[derive(Debug, Clone, Copy)]
pub struct BatchingConfig {
    /// Batch interval in milliseconds.
    ///
    /// Chunks are accumulated and sent every interval_ms.
    pub interval_ms: u64,

    /// Minimum content length to trigger immediate flush.
    ///
    /// If accumulated content exceeds this length, flush immediately.
    pub min_flush_length: usize,
}

impl Default for BatchingConfig {
    fn default() -> Self {
        Self {
            interval_ms: 50,
            min_flush_length: 20,
        }
    }
}

impl BatchingConfig {
    /// Creates a new batching configuration.
    pub const fn new(interval_ms: u64, min_flush_length: usize) -> Self {
        Self {
            interval_ms,
            min_flush_length,
        }
    }

    /// Creates a configuration for immediate streaming (no batching).
    pub const fn immediate() -> Self {
        Self {
            interval_ms: 0,
            min_flush_length: 1,
        }
    }
}

/// Batches stream chunks to reduce IPC overhead.
pub struct ChunkBatcher {
    config: BatchingConfig,
    full_content: String,
    batch_buffer: String,
    full_reasoning: String,
    reasoning_buffer: String,
    last_flush: std::time::Instant,
    final_usage: Option<UsageStats>,
    final_finish_reason: Option<String>,
    finished: bool,
}

impl ChunkBatcher {
    /// Creates a new chunk batcher.
    pub fn new(config: BatchingConfig) -> Self {
        Self {
            config,
            full_content: String::new(),
            batch_buffer: String::new(),
            full_reasoning: String::new(),
            reasoning_buffer: String::new(),
            last_flush: std::time::Instant::now(),
            final_usage: None,
            final_finish_reason: None,
            finished: false,
        }
    }

    /// Processes a chunk and returns events to emit (if any).
    pub fn process_chunk(
        &mut self,
        chunk: &StreamChunk,
        request_id: &str,
    ) -> Option<Vec<LlmEvent>> {
        // Accumulate content and reasoning
        if let Some(ref delta) = chunk.delta {
            if let Some(content) = &delta.content {
                self.full_content.push_str(content);
                self.batch_buffer.push_str(content);
            }
            if let Some(reasoning) = &delta.reasoning_content {
                self.full_reasoning.push_str(reasoning);
                self.reasoning_buffer.push_str(reasoning);
            }

            return self.check_flush(request_id);
        }

        // Save final usage/finish reason
        if chunk.is_final {
            self.final_usage = chunk.usage.clone();
            self.final_finish_reason = chunk.finish_reason.clone();
            self.finished = true;
        }

        None
    }

    /// Checks if batch should be flushed based on config.
    fn check_flush(&mut self, request_id: &str) -> Option<Vec<LlmEvent>> {
        let has_content = !self.batch_buffer.is_empty();
        let has_reasoning = !self.reasoning_buffer.is_empty();

        if !has_content && !has_reasoning {
            return None;
        }

        let now = std::time::Instant::now();
        let elapsed_ms = now.duration_since(self.last_flush).as_millis() as u64;

        // Calculate total buffer size for flush decision
        let total_buffer_size = self.batch_buffer.len() + self.reasoning_buffer.len();

        let should_flush = if self.config.interval_ms == 0 {
            // Immediate mode: flush everything
            true
        } else {
            elapsed_ms >= self.config.interval_ms
                || total_buffer_size >= self.config.min_flush_length
        };

        if should_flush {
            let reasoning_delta = if has_reasoning {
                Some(self.reasoning_buffer.clone())
            } else {
                None
            };

            let events = vec![LlmEvent::Chunk {
                request_id: request_id.to_string(),
                delta: self.batch_buffer.clone(),
                reasoning_delta,
            }];
            self.batch_buffer.clear();
            self.reasoning_buffer.clear();
            self.last_flush = now;
            Some(events)
        } else {
            None
        }
    }

    /// Flushes any remaining buffered content.
    pub fn flush(&mut self, request_id: &str) -> Option<LlmEvent> {
        let has_content = !self.batch_buffer.is_empty();
        let has_reasoning = !self.reasoning_buffer.is_empty();

        if !has_content && !has_reasoning {
            return None;
        }

        let reasoning_delta = if has_reasoning {
            Some(self.reasoning_buffer.clone())
        } else {
            None
        };

        let event = LlmEvent::Chunk {
            request_id: request_id.to_string(),
            delta: self.batch_buffer.clone(),
            reasoning_delta,
        };
        self.batch_buffer.clear();
        self.reasoning_buffer.clear();
        Some(event)
    }

    /// Finishes batching and returns the final chunk if any.
    pub fn finish(&mut self, request_id: &str) -> Option<LlmEvent> {
        self.flush(request_id)
    }

    /// Returns the accumulated full content (consuming the batcher).
    #[must_use]
    pub fn into_full_content(mut self) -> String {
        let content = self.full_content.clone();
        self.full_content.clear();
        content
    }

    /// Returns the full content (borrowed, doesn't consume).
    pub fn full_content(&self) -> String {
        self.full_content.clone()
    }

    /// Returns the final finish reason.
    pub fn finish_reason(&self) -> Option<String> {
        self.final_finish_reason.clone()
    }

    /// Returns whether the stream is finished.
    pub fn is_finished(&self) -> bool {
        self.finished
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batcher_accumulates_content() {
        let config = BatchingConfig::new(100, 50); // High thresholds
        let mut batcher = ChunkBatcher::new(config);

        let chunk = StreamChunk {
            id: "test".to_string(),
            model: "test".to_string(),
            delta: Some(StreamDelta {
                role: None,
                content: Some("Hello".to_string()),
                reasoning_content: None,
            }),
            usage: None,
            finish_reason: None,
            is_final: false,
        };

        assert!(batcher.process_chunk(&chunk, "req-1").is_none());
        assert_eq!(batcher.full_content(), "Hello");
    }

    #[test]
    fn test_batcher_flushes_on_length() {
        let config = BatchingConfig::new(100, 10); // Flush at 10 chars
        let mut batcher = ChunkBatcher::new(config);

        let chunk = StreamChunk {
            id: "test".to_string(),
            model: "test".to_string(),
            delta: Some(StreamDelta {
                role: None,
                content: Some("Hello world! Long text here".to_string()),
                reasoning_content: None,
            }),
            usage: None,
            finish_reason: None,
            is_final: false,
        };

        let events = batcher.process_chunk(&chunk, "req-1");
        assert!(events.is_some());
        assert_eq!(events.unwrap().len(), 1);
    }

    #[test]
    fn test_batcher_flush_returns_content() {
        let mut batcher = ChunkBatcher::new(BatchingConfig::default());
        batcher.batch_buffer = "test content".to_string();

        let event = batcher.flush("req-1");
        assert!(event.is_some());

        if let Some(LlmEvent::Chunk { delta, .. }) = event {
            assert_eq!(delta, "test content");
        } else {
            panic!("Expected Chunk event");
        }
    }

    #[test]
    fn test_batcher_immediate_mode() {
        let config = BatchingConfig::immediate();
        let mut batcher = ChunkBatcher::new(config);

        let chunk = StreamChunk {
            id: "test".to_string(),
            model: "test".to_string(),
            delta: Some(StreamDelta {
                role: None,
                content: Some("Hi".to_string()),
                reasoning_content: None,
            }),
            usage: None,
            finish_reason: None,
            is_final: false,
        };

        let events = batcher.process_chunk(&chunk, "req-1");
        assert!(events.is_some());
    }

    #[test]
    fn test_batcher_with_reasoning_content() {
        let config = BatchingConfig::immediate();
        let mut batcher = ChunkBatcher::new(config);

        // GLM-5 style chunk with reasoning_content instead of content
        let chunk = StreamChunk {
            id: "test".to_string(),
            model: "glm-5".to_string(),
            delta: Some(StreamDelta {
                role: None,
                content: None,
                reasoning_content: Some("Let me think...".to_string()),
            }),
            usage: None,
            finish_reason: None,
            is_final: false,
        };

        let events = batcher.process_chunk(&chunk, "req-1");
        assert!(events.is_some());

        if let Some(events_vec) = events {
            assert_eq!(events_vec.len(), 1);
            if let LlmEvent::Chunk {
                reasoning_delta, ..
            } = &events_vec[0]
            {
                assert_eq!(reasoning_delta, &Some("Let me think...".to_string()));
            } else {
                panic!("Expected Chunk event");
            }
        }
    }
}
