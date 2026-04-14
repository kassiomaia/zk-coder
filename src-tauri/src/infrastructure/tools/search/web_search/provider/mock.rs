//! Test-only mock implementation of `SearchProvider`.
//!
//! `MockProvider` lets the tool-level tests exercise every code path in
//! `WebSearchTool::execute` without touching the network. Three behaviors
//! are supported:
//!
//! 1. Return a canned list of `RawResult`s.
//! 2. Return a specific `SearchProviderError` (rate-limit, unauthorized,
//!    upstream, network) on the next call.
//! 3. Record every `SearchRequest` it received so the test can assert that
//!    the tool passed the expected query / language / site through.

use std::sync::Mutex;

use super::{RawResult, SearchProvider, SearchProviderError, SearchRequest};

/// Canned response the mock will return on the next call.
#[derive(Debug, Clone)]
pub enum MockResponse {
    Ok(Vec<RawResult>),
    RateLimited,
    Unauthorized,
    Upstream(String),
    Network(String),
}

/// Test-only `SearchProvider` with scripted responses and request recording.
pub struct MockProvider {
    script: Mutex<Vec<MockResponse>>,
    observed: Mutex<Vec<SearchRequest>>,
    default_response: MockResponse,
}

impl MockProvider {
    /// Creates a mock that always returns `responses` as its default
    /// successful output when no scripted response is pending.
    pub fn with_results(results: Vec<RawResult>) -> Self {
        Self {
            script: Mutex::new(Vec::new()),
            observed: Mutex::new(Vec::new()),
            default_response: MockResponse::Ok(results),
        }
    }

    /// Queues `response` for the next call. Scripted responses are consumed
    /// FIFO; once exhausted the mock falls back to `default_response`.
    pub fn enqueue(&self, response: MockResponse) {
        self.script
            .lock()
            .expect("mock script lock poisoned")
            .push(response);
    }

    /// Returns a snapshot of every request the mock has observed, in order.
    pub fn observed_requests(&self) -> Vec<SearchRequest> {
        self.observed
            .lock()
            .expect("mock observed lock poisoned")
            .clone()
    }
}

#[async_trait::async_trait]
impl SearchProvider for MockProvider {
    async fn search(&self, req: SearchRequest) -> Result<Vec<RawResult>, SearchProviderError> {
        self.observed
            .lock()
            .expect("mock observed lock poisoned")
            .push(req);

        let mut script = self.script.lock().expect("mock script lock poisoned");
        let response = if script.is_empty() {
            self.default_response.clone()
        } else {
            script.remove(0)
        };
        drop(script);

        match response {
            MockResponse::Ok(results) => Ok(results),
            MockResponse::RateLimited => Err(SearchProviderError::RateLimited),
            MockResponse::Unauthorized => Err(SearchProviderError::Unauthorized),
            MockResponse::Upstream(msg) => Err(SearchProviderError::Upstream(msg)),
            MockResponse::Network(msg) => Err(SearchProviderError::Network(msg)),
        }
    }
}
