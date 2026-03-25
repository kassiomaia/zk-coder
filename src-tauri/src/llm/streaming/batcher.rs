//! Chunk batching for LLM streaming.
//!
//! Batches small chunks together to reduce IPC overhead while
//! maintaining responsive streaming.

use crate::llm::{LlmEvent, StreamChunk, UsageStats};

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
        // Accumulate content
        if let Some(ref delta) = chunk.delta {
            if let Some(content) = &delta.content {
                self.full_content.push_str(content);
                self.batch_buffer.push_str(content);

                return self.check_flush(request_id);
            }
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
        if self.batch_buffer.is_empty() {
            return None;
        }

        let now = std::time::Instant::now();
        let elapsed_ms = now.duration_since(self.last_flush).as_millis() as u64;

        let should_flush = if self.config.interval_ms == 0 {
            // Immediate mode: flush everything
            true
        } else {
            elapsed_ms >= self.config.interval_ms
                || self.batch_buffer.len() >= self.config.min_flush_length
        };

        if should_flush {
            let events = vec![LlmEvent::Chunk {
                request_id: request_id.to_string(),
                delta: self.batch_buffer.clone(),
            }];
            self.batch_buffer.clear();
            self.last_flush = now;
            Some(events)
        } else {
            None
        }
    }

    /// Flushes any remaining buffered content.
    pub fn flush(&mut self, request_id: &str) -> Option<LlmEvent> {
        if self.batch_buffer.is_empty() {
            return None;
        }

        let event = LlmEvent::Chunk {
            request_id: request_id.to_string(),
            delta: self.batch_buffer.clone(),
        };
        self.batch_buffer.clear();
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
    use crate::llm::StreamDelta;

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
            }),
            usage: None,
            finish_reason: None,
            is_final: false,
        };

        let events = batcher.process_chunk(&chunk, "req-1");
        assert!(events.is_some());
    }
}
