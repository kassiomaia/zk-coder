# NLP Implementation Plan

**Project**: Coder AI Desktop Application
**Date**: 2026-02-28
**Status**: Ready for Implementation

---

## Overview

This plan details the implementation of Natural Language Processing (NLP) capabilities for the Coder AI desktop application. The NLP system will enable intelligent understanding of developer commands, maintain context across conversations, detect multiple intents, resolve ambiguities, and understand code-specific language.

### Technology Stack

- **AI Provider**: Anthropic Claude (Claude Opus 4.6, Sonnet 4.6, Haiku 4.6)
- **Backend**: Rust with tokio async runtime
- **Frontend**: Svelte 5 with TypeScript
- **Storage**: SQLite for local context and conversation history
- **Code Analysis**: tree-sitter for code parsing and understanding

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         Frontend (Svelte)                        │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐   │
│  │   Chat UI    │  │  Command     │  │  Code Context        │   │
│  │              │  │  Palette     │  │  Provider            │   │
│  └──────┬───────┘  └──────┬───────┘  └──────────┬───────────┘   │
│         │                  │                     │                │
└─────────┼──────────────────┼─────────────────────┼────────────────┘
          │                  │                     │
          │              Tauri IPC                 │
          │                  │                     │
┌─────────┼──────────────────┼─────────────────────┼────────────────┐
│         │    Rust Backend  │                     │                │
│  ┌──────▼───────┐  ┌──────▼───────┐  ┌──────────▼───────────┐    │
│  │   NLP        │  │   Context    │  │   Code Analysis      │    │
│  │   Engine     │  │   Manager    │  │   (tree-sitter)      │    │
│  └──────┬───────┘  └──────┬───────┘  └──────────┬───────────┘    │
│         │                  │                     │                │
│  ┌──────▼───────┐  ┌──────▼───────┐  ┌──────────▼───────────┐    │
│  │   Intent     │  │   Thread     │  │   Symbol             │    │
│  │   Classifier │  │   Manager    │  │   Resolver           │    │
│  └──────────────┘  └──────────────┘  └──────────────────────┘    │
│         │                  │                     │                │
└─────────┼──────────────────┼─────────────────────┼────────────────┘
          │                  │                     │
          ▼                  ▼                     ▼
   ┌─────────────┐    ┌─────────────┐     ┌─────────────┐
   │   Claude    │    │   SQLite    │     │   Project    │
   │   API       │    │   Database  │     │   Files      │
   └─────────────┘    └─────────────┘     └─────────────┘
```

---

## Task 1.1: Develop Intent Recognition for Developer Commands

### Objective

Build a robust intent recognition system that can identify and categorize developer commands from natural language input.

### Requirements

#### 1.1.1 Intent Taxonomy

```rust
// src-tauri/src/nlp/intent.rs

/// Developer command intents supported by the system.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    // Code generation intents
    GenerateCode,
    GenerateFunction,
    GenerateClass,
    GenerateTest,
    RefactorCode,
    OptimizeCode,

    // Code understanding intents
    ExplainCode,
    AnalyzeCode,
    FindBugs,
    DocumentCode,

    // Navigation intents
    FindFile,
    FindSymbol,
    GotoDefinition,
    FindReferences,

    // Project management intents
    RunTests,
    BuildProject,
    FormatCode,
    LintCode,

    // Git operations
    GitStatus,
    GitCommit,
    GitDiff,
    GitLog,

    // Conversation management
    StartNewThread,
    SwitchContext,
    ClearHistory,

    // Unknown intent
    Unknown,
}

/// Confidence score for intent classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentClassification {
    pub intent: Intent,
    pub confidence: f64,
    pub alternative_intents: Vec<(Intent, f64)>,
}

/// Extracted parameters from the user input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentParams {
    pub language: Option<String>,
    pub file_path: Option<String>,
    pub symbol_name: Option<String>,
    pub code_snippet: Option<String>,
    pub description: Option<String>,
}
```

#### 1.1.2 Intent Classification Engine

**File**: `src-tauri/src/nlp/classifier.rs`

```rust
use crate::nlp::intent::{Intent, IntentClassification, IntentParams};
use crate::ai::claude::ClaudeClient;

pub struct IntentClassifier {
    claude_client: ClaudeClient,
    patterns: IntentPatterns,
}

impl IntentClassifier {
    pub async fn classify(&self, input: &str, context: &ConversationContext) -> IntentClassification {
        // 1. Try pattern matching first (fast path)
        if let Some(intent) = self.patterns.match_input(input) {
            return IntentClassification {
                intent: intent.clone(),
                confidence: 0.95,
                alternative_intents: vec![],
            };
        }

        // 2. Use Claude for complex classification
        let classification = self.classify_with_ai(input, context).await;

        // 3. Apply confidence boosting based on context
        self.boost_confidence(classification, context)
    }

    async fn classify_with_ai(&self, input: &str, context: &ConversationContext) -> IntentClassification {
        let system_prompt = self.build_classification_prompt(context);
        let response = self.claude_client
            .complete(&system_prompt, input)
            .await
            .unwrap();

        self.parse_classification_response(&response)
    }

    fn build_classification_prompt(&self, context: &ConversationContext) -> String {
        format!(
            "You are an intent classifier for a developer AI assistant.
            Classify the user's input into one of these intents: {:?}
            Consider the conversation context: {:?}
            Return JSON with: intent, confidence (0-1), and alternatives.",
            Intent::variants(),
            context
        )
    }
}
```

#### 1.1.3 Pattern Matching System

```rust
// src-tauri/src/nlp/patterns.rs

use regex::Regex;
use lazy_static::lazy_static;

pub struct IntentPatterns {
    patterns: Vec<(Intent, Vec<Regex>)>,
}

impl IntentPatterns {
    pub fn new() -> Self {
        IntentPatterns {
            patterns: vec![
                (Intent::GenerateCode, vec![
                    Regex::new(r"(?i)^(generate|create|write)\s+(code|function|class)").unwrap(),
                    Regex::new(r"(?i)^(help\s+me\s+)?(code|implement|build)").unwrap(),
                ]),
                (Intent::ExplainCode, vec![
                    Regex::new(r"(?i)^(explain|what\s+does|how\s+does)").unwrap(),
                    Regex::new(r"(?i)^tell\s+me\s+about").unwrap(),
                ]),
                (Intent::FindBugs, vec![
                    Regex::new(r"(?i)(find|detect|spot)\s+(bugs?|issues?|errors?)").unwrap(),
                    Regex::new(r"(?i)^what'?s\s+wrong").unwrap(),
                ]),
                (Intent::RefactorCode, vec![
                    Regex::new(r"(?i)^(refactor|improve|clean\s+up)").unwrap(),
                    Regex::new(r"(?i)make\s+(this|code)\s+(better|cleaner)").unwrap(),
                ]),
                // ... more patterns
            ],
        }
    }

    pub fn match_input(&self, input: &str) -> Option<&Intent> {
        for (intent, patterns) in &self.patterns {
            for pattern in patterns {
                if pattern.is_match(input) {
                    return Some(intent);
                }
            }
        }
        None
    }
}
```

#### 1.1.4 Frontend Integration

```typescript
// src-ui/src/lib/types/nlp.ts

export interface Intent {
  type: IntentType;
  confidence: number;
  params: IntentParams;
}

export enum IntentType {
  GENERATE_CODE = 'generate_code',
  EXPLAIN_CODE = 'explain_code',
  FIND_BUGS = 'find_bugs',
  REFACTOR_CODE = 'refactor_code',
  OPTIMIZE_CODE = 'optimize_code',
  FIND_FILE = 'find_file',
  FIND_SYMBOL = 'find_symbol',
  RUN_TESTS = 'run_tests',
  BUILD_PROJECT = 'build_project',
  FORMAT_CODE = 'format_code',
  GIT_STATUS = 'git_status',
  GIT_COMMIT = 'git_commit',
  START_NEW_THREAD = 'start_new_thread',
  UNKNOWN = 'unknown',
}

export interface IntentParams {
  language?: string;
  filePath?: string;
  symbolName?: string;
  codeSnippet?: string;
  description?: string;
}
```

```svelte
<!-- src-ui/src/lib/components/chat/input/IntentIndicator.svelte -->
<script lang="ts">
  import styles from './IntentIndicator.module.css';
  import type { Intent } from '$lib/types/nlp';

  interface Props {
    intent: Intent | null;
  }

  let { intent }: Props = $props();

  const intentLabels: Record<IntentType, string> = {
    [IntentType.GENERATE_CODE]: 'Generate Code',
    [IntentType.EXPLAIN_CODE]: 'Explain',
    [IntentType.FIND_BUGS]: 'Find Bugs',
    [IntentType.REFACTOR_CODE]: 'Refactor',
    [IntentType.OPTIMIZE_CODE]: 'Optimize',
    [IntentType.FIND_FILE]: 'Find File',
    [IntentType.FIND_SYMBOL]: 'Find Symbol',
    [IntentType.RUN_TESTS]: 'Run Tests',
    [IntentType.BUILD_PROJECT]: 'Build',
    [IntentType.FORMAT_CODE]: 'Format',
    [IntentType.GIT_STATUS]: 'Git Status',
    [IntentType.GIT_COMMIT]: 'Git Commit',
    [IntentType.START_NEW_THREAD]: 'New Thread',
    [IntentType.UNKNOWN]: 'Command',
  };

  let confidenceLevel = $derived(() => {
    if (!intent) return null;
    if (intent.confidence > 0.8) return 'high';
    if (intent.confidence > 0.5) return 'medium';
    return 'low';
  });
</script>

{#if intent && intent.type !== IntentType.UNKNOWN}
  <div class={[styles.indicator, styles[confidenceLevel!]].filter(Boolean).join(' ')}>
    <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
      <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z" />
    </svg>
    <span>{intentLabels[intent.type]}</span>
    <span class={styles.confidence}>{Math.round(intent.confidence * 100)}%</span>
  </div>
{/if}
```

---

## Task 1.2: Implement Context Preservation Across Conversation Threads

### Objective

Maintain conversation context across threads, enabling the AI to remember and reference previous discussions, code changes, and user preferences.

### Requirements

#### 1.2.1 Context Data Model

```rust
// src-tauri/src/nlp/context.rs

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Represents a single context window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWindow {
    pub id: String,
    pub conversation_id: String,
    pub thread_id: Option<String>,
    pub messages: Vec<ContextMessage>,
    pub metadata: ContextMetadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A message within the context window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMessage {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub tokens: Option<usize>,
    pub embedding: Option<Vec<f32>>,  // For semantic search
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// Metadata about the context window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMetadata {
    pub current_file: Option<FileInfo>,
    pub active_symbols: Vec<SymbolReference>,
    pub project_state: ProjectState,
    pub user_preferences: UserPreferences,
    pub summary: Option<String>,  // Compressed context for older messages
}

/// Information about a file in context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub language: String,
    pub line_count: usize,
    pub last_modified: DateTime<Utc>,
}

/// Reference to a code symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolReference {
    pub name: String,
    pub kind: SymbolKind,
    pub file_path: String,
    pub line_range: (usize, usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Class,
    Variable,
    Constant,
    Interface,
    TypeAlias,
}

/// Current project state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectState {
    pub git_branch: Option<String>,
    pub build_status: BuildStatus,
    pub test_results: Option<TestResults>,
    pub errors: Vec<BuildError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildStatus {
    Unknown,
    Building,
    Success,
    Failed,
}
```

#### 1.2.2 Context Manager

```rust
// src-tauri/src/nlp/manager.rs

use crate::nlp::context::{ContextWindow, ContextMessage};
use crate::storage::context_store::ContextStore;

pub struct ContextManager {
    store: ContextStore,
    max_tokens: usize,
    summary_threshold: usize,
}

impl ContextManager {
    pub fn new(store: ContextStore) -> Self {
        ContextManager {
            store,
            max_tokens: 100_000,  // Claude's context window
            summary_threshold: 50_000,  // Summarize at 50% capacity
        }
    }

    /// Add a message to the context window.
    pub async fn add_message(
        &self,
        conversation_id: &str,
        message: ContextMessage,
    ) -> Result<(), ContextError> {
        let mut window = self.store.load_context(conversation_id).await?;

        window.messages.push(message);
        window.updated_at = Utc::now();

        // Check if we need to compress
        if self.estimate_tokens(&window) > self.summary_threshold {
            window = self.compress_context(window).await?;
        }

        self.store.save_context(&window).await?;
        Ok(())
    }

    /// Get relevant context for a new query.
    pub async fn get_relevant_context(
        &self,
        conversation_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ContextMessage>, ContextError> {
        let window = self.store.load_context(conversation_id).await?;

        // Use semantic search to find relevant messages
        let query_embedding = self.generate_embedding(query).await?;
        let mut scored: Vec<_> = window.messages
            .iter()
            .filter_map(|m| {
                m.embedding.as_ref().map(|emb| {
                    (m, cosine_similarity(&query_embedding, emb))
                })
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        Ok(scored.into_iter()
            .take(limit)
            .map(|(m, _)| m.clone())
            .collect())
    }

    /// Compress old context using AI summarization.
    async fn compress_context(
        &self,
        mut window: ContextWindow,
    ) -> Result<ContextWindow, ContextError> {
        let messages_to_compress: Vec<_> = window.messages
            .iter()
            .take(window.messages.len() / 2)
            .cloned()
            .collect();

        let summary = self.generate_summary(&messages_to_compress).await?;

        window.metadata.summary = Some(summary);
        window.messages = window.messages
            .into_iter()
            .skip(messages_to_compress.len())
            .collect();

        Ok(window)
    }

    async fn generate_summary(&self, messages: &[ContextMessage]) -> Result<String, ContextError> {
        let prompt = self.build_summary_prompt(messages);

        // Use Claude for summarization
        let client = ClaudeClient::new();
        let response = client.complete(
            "Summarize the following conversation concisely, preserving key technical details and decisions.",
            &prompt
        ).await?;

        Ok(response)
    }
}
```

#### 1.2.3 Context Store (SQLite)

```rust
// src-tauri/src/storage/context_store.rs

use rusqlite::{Connection, params};
use crate::nlp::context::ContextWindow;

pub struct ContextStore {
    conn: Connection,
}

impl ContextStore {
    pub fn new(db_path: &str) -> Result<Self, StoreError> {
        let conn = Connection::open(db_path)?;
        Self::init_schema(&conn)?;
        Ok(ContextStore { conn })
    }

    fn init_schema(conn: &Connection) -> Result<(), StoreError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS context_windows (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                thread_id TEXT,
                metadata TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (conversation_id) REFERENCES conversations(id),
                FOREIGN KEY (thread_id) REFERENCES threads(id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS context_messages (
                id TEXT PRIMARY KEY,
                context_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                tokens INTEGER,
                embedding BLOB,
                FOREIGN KEY (context_id) REFERENCES context_windows(id)
            )",
            [],
        )?;

        // Create index for semantic search
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_context_messages_context_id
             ON context_messages(context_id)",
            [],
        )?;

        Ok(())
    }

    pub async fn load_context(&self, conversation_id: &str) -> Result<ContextWindow, StoreError> {
        // Implementation for loading context
    }

    pub async fn save_context(&self, window: &ContextWindow) -> Result<(), StoreError> {
        // Implementation for saving context
    }
}
```

#### 1.2.4 Frontend Context Display

```svelte
<!-- src-ui/src/lib/components/chat/ContextIndicator.svelte -->
<script lang="ts">
  import styles from './ContextIndicator.module.css';

  interface Props {
    fileCount: number;
    symbolCount: number;
    summary?: string;
  }

  let { fileCount, symbolCount, summary }: Props = $props();

  let showSummary = $state(false);
</script>

<div class={styles.indicator}>
  <button
    class={styles.toggle}
    onclick={() => showSummary = !showSummary}
    aria-label="Toggle context details"
  >
    <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
      <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z" />
    </svg>
    <span>Context Active</span>
    <span class={styles.count}>{fileCount} files, {symbolCount} symbols</span>
  </button>

  {#if showSummary && summary}
    <div class={styles.summary}>
      <h4>Conversation Summary</h4>
      <p>{summary}</p>
    </div>
  {/if}
</div>
```

---

## Task 1.3: Create Multi-Intent Detection for Complex Developer Queries

### Objective

Detect and handle multiple intents within a single complex query, such as "find bugs in the authentication code and write tests for the fixed version."

### Requirements

#### 1.3.1 Multi-Intent Data Model

```rust
// src-tauri/src/nlp/multi_intent.rs

use crate::nlp::intent::{Intent, IntentParams};
use serde::{Deserialize, Serialize};

/// Represents a detected intent with its position in the input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedIntent {
    pub intent: Intent,
    pub params: IntentParams,
    pub span: TextSpan,
    pub confidence: f64,
}

/// A span of text in the original input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSpan {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// Multiple intents detected in a single input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiIntentResult {
    pub intents: Vec<DetectedIntent>,
    pub relationships: Vec<IntentRelationship>,
    pub execution_order: Vec<usize>,
}

/// Relationships between detected intents.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentRelationship {
    Sequential,     // First intent must complete before second
    Parallel,       // Intents can execute simultaneously
    Dependent,      // Second intent depends on results of first
    Exclusive,      // Only one intent should be executed
}
```

#### 1.3.2 Multi-Intent Detector

```rust
// src-tauri/src/nlp/multi_detector.rs

use crate::nlp::multi_intent::{DetectedIntent, MultiIntentResult, IntentRelationship};
use crate::ai::claude::ClaudeClient;

pub struct MultiIntentDetector {
    claude: ClaudeClient,
}

impl MultiIntentDetector {
    pub async fn detect(&self, input: &str) -> Result<MultiIntentResult, DetectorError> {
        // Step 1: Check for obvious multi-intent markers
        if self.has_conjunctions(input) {
            return self.detect_complex(input).await;
        }

        // Step 2: Use AI to detect complex intents
        let result = self.analyze_with_ai(input).await?;

        // Step 3: Determine relationships between intents
        Ok(self.determine_relationships(result))
    }

    fn has_conjunctions(&self, input: &str) -> bool {
        let conjunctions = [" and ", " then ", " after ", " before ", " while "];
        conjunctions.iter().any(|c| input.to_lowercase().contains(c))
    }

    async fn detect_complex(&self, input: &str) -> Result<MultiIntentResult, DetectorError> {
        // Split on common conjunctions and classify each part
        let parts = self.split_on_conjunctions(input);
        let mut intents = Vec::new();

        for part in parts {
            let classification = self.classifier.classify(&part.text, context).await;
            intents.push(DetectedIntent {
                intent: classification.intent,
                params: self.extract_params(&part.text),
                span: part,
                confidence: classification.confidence,
            });
        }

        Ok(MultiIntentResult {
            intents,
            relationships: vec![],
            execution_order: (0..intents.len()).collect(),
        })
    }

    async fn analyze_with_ai(&self, input: &str) -> Result<MultiIntentResult, DetectorError> {
        let prompt = format!(
            "Analyze this developer query for multiple intents: '{}'
            Return a JSON response with:
            - detected intents (with text spans)
            - relationships between intents (sequential, parallel, dependent, exclusive)
            - recommended execution order",
            input
        );

        let response = self.claude.complete(
            "You are a multi-intent detector for developer commands.",
            &prompt
        ).await?;

        self.parse_multi_intent_response(&response)
    }

    fn determine_relationships(&self, mut result: MultiIntentResult) -> MultiIntentResult {
        // Analyze intent relationships
        for i in 0..result.intents.len() {
            for j in (i + 1)..result.intents.len() {
                let relationship = self.infer_relationship(
                    &result.intents[i],
                    &result.intents[j]
                );
                result.relationships.push(relationship);
            }
        }
        result
    }

    fn infer_relationship(
        &self,
        first: &DetectedIntent,
        second: &DetectedIntent
    ) -> IntentRelationship {
        // Look for relationship markers in the text between intents
        let text_between = &first.span.text[first.span.end..second.span.start];
        let text_lower = text_between.to_lowercase();

        if text_lower.contains(" then ") || text_lower.contains(" after ") {
            IntentRelationship::Sequential
        } else if text_lower.contains(" and ") || text_lower.contains(" while ") {
            IntentRelationship::Parallel
        } else if text_lower.contains(" using ") || text_lower.contains(" with ") {
            IntentRelationship::Dependent
        } else {
            IntentRelationship::Sequential  // Default
        }
    }
}
```

#### 1.3.3 Execution Planner

```rust
// src-tauri/src/nlp/execution_planner.rs

use crate::nlp::multi_intent::{MultiIntentResult, IntentRelationship};
use std::collections::HashMap;

pub struct ExecutionPlanner;

impl ExecutionPlanner {
    pub fn create_execution_plan(
        &self,
        result: &MultiIntentResult
    ) -> ExecutionPlan {
        let mut levels = Vec::new();
        let mut executed = std::collections::HashSet::new();
        let mut intent_map: HashMap<usize, &IntentRelationship> = HashMap::new();

        for relationship in &result.relationships {
            // Build dependency graph
        }

        while executed.len() < result.intents.len() {
            let mut current_level = Vec::new();

            for (i, intent) in result.intents.iter().enumerate() {
                if executed.contains(&i) {
                    continue;
                }

                if self.can_execute(&result.relationships, i, &executed) {
                    current_level.push(i);
                    executed.insert(i);
                }
            }

            if current_level.is_empty() {
                break;  // Circular dependency or no executable intents
            }

            levels.push(current_level);
        }

        ExecutionPlan { levels }
    }

    fn can_execute(
        &self,
        relationships: &[IntentRelationship],
        intent_index: usize,
        executed: &std::collections::HashSet<usize>
    ) -> bool {
        // Check if all dependencies are satisfied
        true  // Simplified
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub levels: Vec<Vec<usize>>,  // Each level contains intents that can run in parallel
}
```

#### 1.3.4 Frontend Multi-Intent Display

```svelte
<!-- src-ui/src/lib/components/chat/MultiIntentIndicator.svelte -->
<script lang="ts">
  import styles from './MultiIntentIndicator.module.css';
  import type { MultiIntentResult } from '$lib/types/nlp';

  interface Props {
    result: MultiIntentResult;
  }

  let { result }: Props = $props();

  let expanded = $state(false);
</script>

<div class={styles.container}>
  <button
    class={styles.header}
    onclick={() => expanded = !expanded}
  >
    <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
      <path d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm-5 14H7v-2h7v2zm3-4H7v-2h10v2zm0-4H7V7h10v2z" />
    </svg>
    <span>{result.intents.length} intents detected</span>
    <svg class={[styles.chevron, expanded && styles.open].filter(Boolean).join('')}
         width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
      <path d="M7.41 8.59L12 13.17l4.59-4.58L18 10l-6 6-6-6 1.41-1.41z" />
    </svg>
  </button>

  {#if expanded}
    <div class={styles.intents}>
      {#each result.intents as intent, index (index)}
        <div class={styles.intent}>
          <span class={styles.index}>{index + 1}</span>
          <span class={styles.name}>{intent.intent}</span>
          <span class={styles.confidence}>{Math.round(intent.confidence * 100)}%</span>
        </div>
      {/each}

      {#if result.execution_order.length > 1}
        <div class={styles.executionPlan}>
          <h4>Execution Plan</h4>
          <p>Intents will be executed in the recommended order above.</p>
        </div>
      {/if}
    </div>
  {/if}
</div>
```

---

## Task 1.4: Build Ambiguity Resolution for Technical Language

### Objective

Detect ambiguities in technical language and resolve them through clarification questions, context inference, and user confirmation.

### Requirements

#### 1.4.1 Ambiguity Detection

```rust
// src-tauri/src/nlp/ambiguity.rs

use serde::{Deserialize, Serialize};

/// Types of ambiguities that can be detected.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmbiguityType {
    MissingFile,
    MissingSymbol,
    AmbiguousReference,
    MultipleMatches,
    UnclearScope,
    MissingParameter,
    VersionConflict,
}

/// A detected ambiguity in user input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ambiguity {
    pub ambiguity_type: AmbiguityType,
    pub span: TextSpan,
    pub severity: AmbiguitySeverity,
    pub clarification: ClarificationRequest,
    pub suggestions: Vec<AmbiguitySuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmbiguitySeverity {
    Low,      // Can guess with high confidence
    Medium,   // Should confirm but have likely answer
    High,     // Must clarify before proceeding
}

/// Request for clarification from the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarificationRequest {
    pub question: String,
    pub options: Vec<ClarificationOption>,
    pub allows_freeform: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarificationOption {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
}

/// A suggested resolution for an ambiguity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmbiguitySuggestion {
    pub value: String,
    pub confidence: f64,
    pub reason: String,
}
```

#### 1.4.2 Ambiguity Resolver

```rust
// src-tauri/src/nlp/ambiguity_resolver.rs

use crate::nlp::ambiguity::{Ambiguity, AmbiguityType, AmbiguitySuggestion};
use crate::code_analysis::symbol_resolver::SymbolResolver;

pub struct AmbiguityResolver {
    symbol_resolver: SymbolResolver,
}

impl AmbiguityResolver {
    pub async fn detect_ambiguities(
        &self,
        input: &str,
        context: &ConversationContext
    ) -> Result<Vec<Ambiguity>, ResolverError> {
        let mut ambiguities = Vec::new();

        // Check for file reference ambiguities
        if let Some(file_ambiguity) = self.check_file_ambiguities(input, context).await? {
            ambiguities.push(file_ambiguity);
        }

        // Check for symbol reference ambiguities
        if let Some(symbol_ambiguity) = self.check_symbol_ambiguities(input, context).await? {
            ambiguities.push(symbol_ambiguity);
        }

        // Check for scope ambiguities
        if let Some(scope_ambiguity) = self.check_scope_ambiguities(input, context).await? {
            ambiguities.push(scope_ambiguity);
        }

        Ok(ambiguities)
    }

    async fn check_file_ambiguities(
        &self,
        input: &str,
        context: &ConversationContext
    ) -> Result<Option<Ambiguity>, ResolverError> {
        // Extract file references from input
        let file_refs = self.extract_file_references(input);

        for file_ref in file_refs {
            // Try to resolve the file reference
            let matches = self.symbol_resolver.find_files(&file_ref, context).await?;

            match matches.len() {
                0 => {
                    return Ok(Some(Ambiguity {
                        ambiguity_type: AmbiguityType::MissingFile,
                        span: file_ref.span,
                        severity: AmbiguitySeverity::High,
                        clarification: ClarificationRequest {
                            question: format!("File '{}' not found. Did you mean:", file_ref.text),
                            options: self.generate_similar_file_options(&file_ref.text, context).await?,
                            allows_freeform: true,
                        },
                        suggestions: vec![],
                    }));
                }
                1 => {
                    // Single match - no ambiguity
                    continue;
                }
                _ => {
                    // Multiple matches
                    return Ok(Some(Ambiguity {
                        ambiguity_type: AmbiguityType::MultipleMatches,
                        span: file_ref.span,
                        severity: AmbiguitySeverity::Medium,
                        clarification: ClarificationRequest {
                            question: format!("Multiple files match '{}'. Which one?", file_ref.text),
                            options: matches.iter().take(5).map(|m| ClarificationOption {
                                value: m.path.clone(),
                                label: m.path.clone(),
                                description: Some(format!("in {}", m.directory)),
                            }).collect(),
                            allows_freeform: true,
                        },
                        suggestions: matches.into_iter().map(|m| AmbiguitySuggestion {
                            value: m.path.clone(),
                            confidence: m.relevance_score,
                            reason: "matches your reference".to_string(),
                        }).collect(),
                    }));
                }
            }
        }

        Ok(None)
    }

    async fn check_symbol_ambiguities(
        &self,
        input: &str,
        context: &ConversationContext
    ) -> Result<Option<Ambiguity>, ResolverError> {
        // Similar logic for symbol references
        Ok(None)
    }

    fn extract_file_references(&self, input: &str) -> Vec<FileReference> {
        // Use regex to find file references like "auth.ts", "components/Button.svelte"
        let re = regex::Regex::new(r"[a-zA-Z0-9_/\\]+\.[a-zA-Z0-9]+").unwrap();

        re.find_iter(input)
            .map(|m| FileReference {
                text: m.as_str().to_string(),
                span: TextSpan {
                    start: m.start(),
                    end: m.end(),
                    text: m.as_str().to_string(),
                },
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
struct FileReference {
    text: String,
    span: TextSpan,
}
```

#### 1.4.3 Context-Aware Resolution

```rust
// src-tauri/src/nlp/context_resolver.rs

use crate::nlp::context::ContextWindow;

pub struct ContextResolver {
    window: ContextWindow,
}

impl ContextResolver {
    /// Try to resolve ambiguity using conversation context.
    pub fn resolve_with_context(&self, ambiguity: &Ambiguity) -> Option<AmbiguitySuggestion> {
        match ambiguity.ambiguity_type {
            AmbiguityType::MissingFile => {
                // Check if file was mentioned in conversation
                self.find_recent_file_mention()
            }
            AmbiguityType::MissingSymbol => {
                // Check if symbol was discussed
                self.find_recent_symbol_mention()
            }
            AmbiguityType::AmbiguousReference => {
                // Use context to disambiguate
                self.disambiguate_from_context(ambiguity)
            }
            _ => None,
        }
    }

    fn find_recent_file_mention(&self) -> Option<AmbiguitySuggestion> {
        // Search through recent messages for file references
        for message in self.window.messages.iter().rev().take(10) {
            // Extract and return file references
        }
        None
    }

    fn disambiguate_from_context(&self, ambiguity: &Ambiguity) -> Option<AmbiguitySuggestion> {
        // Analyze conversation to infer most likely meaning
        None
    }
}
```

#### 1.4.4 Frontend Clarification UI

```svelte
<!-- src-ui/src/lib/components/chat/ClarificationPrompt.svelte -->
<script lang="ts">
  import styles from './ClarificationPrompt.module.css';
  import type { Ambiguity } from '$lib/types/nlp';

  interface Props {
    ambiguity: Ambiguity;
    onResolve: (value: string) => void;
  }

  let { ambiguity, onResolve }: Props = $props();

  let selectedOption = $state<string | null>(null);
  let customValue = $state('');
  let showCustom = $state(false);
</script>

<div class={[styles.prompt, styles[ambiguity.severity]].filter(Boolean).join(' ')}>
  <div class={styles.header}>
    <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
      <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 17h-2v-2h2v2zm2.07-7.75l-.9.92C13.45 12.9 13 13.5 13 15h-2v-.5c0-1.1.45-2.1 1.17-2.83l1.24-1.26c.37-.36.59-.86.59-1.41 0-1.1-.9-2-2-2s-2 .9-2 2H8c0-2.21 1.79-4 4-4s4 1.79 4 4c0 .88-.36 1.68-.93 2.25z" />
    </svg>
    <span>Clarification needed</span>
  </div>

  <p class={styles.question}>{ambiguity.clarification.question}</p>

  <div class={styles.options}>
    {#each ambiguity.clarification.options as option}
      <button
        class={[styles.option, selectedOption === option.value && styles.selected].filter(Boolean).join('')}
        onclick={() => {
          selectedOption = option.value;
          showCustom = false;
        }}
      >
        <span class={styles.label}>{option.label}</span>
        {#if option.description}
          <span class={styles.description}>{option.description}</span>
        {/if}
      </button>
    {/each}

    {#if ambiguity.clarification.allows_freeform}
      <button
        class={[styles.option, styles.custom, showCustom && styles.selected].filter(Boolean).join('')}
        onclick={() => {
          showCustom = true;
          selectedOption = null;
        }}
      >
        <span class={styles.label}>Something else...</span>
      </button>

      {#if showCustom}
        <input
          type="text"
          class={styles.customInput}
          placeholder="Type your answer..."
          bind:value={customValue}
        />
      {/if}
    {/if}
  </div>

  {#if ambiguity.suggestions.length > 0}
    <div class={styles.suggestions}>
      <span class={styles.suggestionsLabel}>Suggested:</span>
      {#each ambiguity.suggestions as suggestion}
        <button
          class={styles.suggestion}
          onclick={() => onResolve(suggestion.value.clone())}
        >
          {suggestion.value}
          <span class={styles.confidence}>{Math.round(suggestion.confidence * 100)}%</span>
        </button>
      {/each}
    </div>
  {/if}

  <div class={styles.actions}>
    <button
      class={styles.resolveButton}
      disabled={!selectedOption && (!showCustom || !customValue)}
      onclick={() => onResolve(selectedOption || customValue)}
    >
      Continue
    </button>
  </div>
</div>
```

---

## Task 1.5: Develop Code-Specific Language Understanding Capabilities

### Objective

Build specialized NLP capabilities for understanding code, programming languages, frameworks, and developer-specific terminology.

### Requirements

#### 1.5.1 Code Understanding Models

```rust
// src-tauri/src/nlp/code_understanding.rs

use tree_sitter::{Parser, Language, Node};
use serde::{Deserialize, Serialize};

/// Information extracted from code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeUnderstanding {
    pub language: ProgrammingLanguage,
    pub structure: CodeStructure,
    pub symbols: Vec<SymbolInfo>,
    pub dependencies: Vec<Dependency>,
    pub patterns: Vec<CodePattern>,
    pub complexity: ComplexityMetrics,
}

/// Programming language detection and info.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProgrammingLanguage {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
    Cpp,
    C,
    Ruby,
    PHP,
    Swift,
    Kotlin,
    Unknown,
}

/// Structure of the code file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeStructure {
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
    pub interfaces: Vec<InterfaceInfo>,
    pub imports: Vec<Import>,
    pub exports: Vec<Export>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    pub visibility: Visibility,
    pub is_async: bool,
    pub is_generator: bool,
    pub line_range: (usize, usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    pub name: String,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub methods: Vec<FunctionInfo>,
    pub properties: Vec<PropertyInfo>,
    pub line_range: (usize, usize),
}

/// Symbols extracted from code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub defining_scope: String,
    pub references: Vec<SymbolReference>,
    pub documentation: Option<String>,
}

/// Code patterns detected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CodePattern {
    Singleton,
    Factory,
    Observer,
    PromiseChain,
    AsyncAwait,
    ErrorHandling(ErrorHandlingPattern),
    DataStructure(DataStructurePattern),
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorHandlingPattern {
    TryCatch,
    ResultType,
    OptionType,
    ErrorCallback,
    Custom,
}

/// Complexity metrics for code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    pub cyclomatic_complexity: usize,
    pub lines_of_code: usize,
    pub nesting_depth: usize,
    pub parameter_count: usize,
}
```

#### 1.5.2 Code Analysis Engine

```rust
// src-tauri/src/nlp/code_analyzer.rs

use tree_sitter::{Parser, Node};
use crate::nlp::code_understanding::*;

pub struct CodeAnalyzer {
    parsers: std::collections::HashMap<ProgrammingLanguage, Parser>,
}

impl CodeAnalyzer {
    pub fn new() -> Result<Self, AnalyzerError> {
        let mut parsers = std::collections::HashMap::new();

        // Initialize parsers for supported languages
        parsers.insert(ProgrammingLanguage::Rust, Self::create_rust_parser()?);
        parsers.insert(ProgrammingLanguage::TypeScript, Self::create_ts_parser()?);
        parsers.insert(ProgrammingLanguage::JavaScript, Self::create_js_parser()?);
        parsers.insert(ProgrammingLanguage::Python, Self::create_python_parser()?);

        Ok(CodeAnalyzer { parsers })
    }

    pub fn analyze(&self, code: &str, language: ProgrammingLanguage) -> Result<CodeUnderstanding, AnalyzerError> {
        let parser = self.parsers.get(&language)
            .ok_or(AnalyzerError::UnsupportedLanguage)?;

        let tree = parser.parse(code, None)
            .ok_or(AnalyzerError::ParseError)?;

        let root = tree.root_node();

        Ok(CodeUnderstanding {
            language,
            structure: self.extract_structure(&root, code)?,
            symbols: self.extract_symbols(&root, code)?,
            dependencies: self.extract_dependencies(&root, code)?,
            patterns: self.detect_patterns(&root, code)?,
            complexity: self.calculate_complexity(&root, code)?,
        })
    }

    fn extract_structure(&self, root: &Node, source: &str) -> Result<CodeStructure, AnalyzerError> {
        let mut functions = Vec::new();
        let mut classes = Vec::new();
        let mut interfaces = Vec::new();
        let mut imports = Vec::new();
        let mut exports = Vec::new();

        // Traverse AST to extract structure
        self.traverse(root, source, &mut |node, source| {
            match node.kind() {
                "function_declaration" | "function_definition" => {
                    if let Some(func) = self.extract_function(node, source) {
                        functions.push(func);
                    }
                }
                "class_declaration" | "class_definition" => {
                    if let Some(class) = self.extract_class(node, source) {
                        classes.push(class);
                    }
                }
                "import_statement" | "import_declaration" => {
                    if let Some(import) = self.extract_import(node, source) {
                        imports.push(import);
                    }
                }
                _ => {}
            }
        });

        Ok(CodeStructure {
            functions,
            classes,
            interfaces,
            imports,
            exports,
        })
    }

    fn detect_patterns(&self, root: &Node, source: &str) -> Result<Vec<CodePattern>, AnalyzerError> {
        let mut patterns = Vec::new();

        // Detect async/await patterns
        if self.contains_async_await(root) {
            patterns.push(CodePattern::AsyncAwait);
        }

        // Detect error handling patterns
        if let Some(pattern) = self.detect_error_handling(root) {
            patterns.push(CodePattern::ErrorHandling(pattern));
        }

        // Detect design patterns
        patterns.extend(self.detect_design_patterns(root, source)?);

        Ok(patterns)
    }

    fn calculate_complexity(&self, root: &Node, source: &str) -> Result<ComplexityMetrics, AnalyzerError> {
        let mut complexity = 1;
        let mut nesting_depth = 0;
        let mut max_nesting = 0;

        self.traverse(root, source, &mut |node, _| {
            match node.kind() {
                "if_statement" | "while_statement" | "for_statement" | "match_statement" => {
                    complexity += 1;
                    nesting_depth += 1;
                    max_nesting = max_nesting.max(nesting_depth);
                }
                _ => {}
            }
        });

        Ok(ComplexityMetrics {
            cyclomatic_complexity: complexity,
            lines_of_code: source.lines().count(),
            nesting_depth: max_nesting,
            parameter_count: self.count_parameters(root),
        })
    }
}
```

#### 1.5.3 Natural Language to Code Mapping

```rust
// src-tauri/src/nlp/nl_to_code.rs

use crate::nlp::code_understanding::*;

/// Maps natural language descriptions to code concepts.
pub struct NLToCodeMapper {
    concept_mappings: Vec<ConceptMapping>,
}

#[derive(Debug, Clone)]
struct ConceptMapping {
    keywords: Vec<String>,
    concept: CodeConcept,
    confidence: f64,
}

#[derive(Debug, Clone)]
enum CodeConcept {
    DataType(String),
    DesignPattern(String),
    Algorithm(String),
    Library(String),
    Framework(String),
}

impl NLToCodeMapper {
    pub fn new() -> Self {
        NLToCodeMapper {
            concept_mappings: Self::build_mappings(),
        }
    }

    fn build_mappings() -> Vec<ConceptMapping> {
        vec![
            // Data types
            ConceptMapping {
                keywords: vec!["list".to_string(), "array".to_string(), "sequence".to_string()],
                concept: CodeConcept::DataType("array".to_string()),
                confidence: 0.9,
            },
            ConceptMapping {
                keywords: vec!["map".to_string(), "dictionary".to_string(), "hash".to_string(), "object".to_string()],
                concept: CodeConcept::DataType("object".to_string()),
                confidence: 0.9,
            },
            ConceptMapping {
                keywords: vec!["set".to_string(), "unique".to_string()],
                concept: CodeConcept::DataType("set".to_string()),
                confidence: 0.85,
            },

            // Design patterns
            ConceptMapping {
                keywords: vec!["singleton".to_string(), "single instance".to_string()],
                concept: CodeConcept::DesignPattern("Singleton".to_string()),
                confidence: 0.95,
            },
            ConceptMapping {
                keywords: vec!["factory".to_string(), "create".to_string(), "builder".to_string()],
                concept: CodeConcept::DesignPattern("Factory".to_string()),
                confidence: 0.9,
            },
            ConceptMapping {
                keywords: vec!["observer".to_string(), "subscribe".to_string(), "event".to_string()],
                concept: CodeConcept::DesignPattern("Observer".to_string()),
                confidence: 0.9,
            },

            // Algorithms
            ConceptMapping {
                keywords: vec!["sort".to_string(), "order".to_string()],
                concept: CodeConcept::Algorithm("sort".to_string()),
                confidence: 0.9,
            },
            ConceptMapping {
                keywords: vec!["search".to_string(), "find".to_string(), "lookup".to_string()],
                concept: CodeConcept::Algorithm("search".to_string()),
                confidence: 0.9,
            },
            ConceptMapping {
                keywords: vec!["filter".to_string(), "where".to_string()],
                concept: CodeConcept::Algorithm("filter".to_string()),
                confidence: 0.9,
            },

            // Libraries and frameworks
            ConceptMapping {
                keywords: vec!["react".to_string(), "component".to_string(), "hook".to_string()],
                concept: CodeConcept::Framework("React".to_string()),
                confidence: 0.85,
            },
            ConceptMapping {
                keywords: vec!["svelte".to_string(), "store".to_string(), "reactive".to_string()],
                concept: CodeConcept::Framework("Svelte".to_string()),
                confidence: 0.85,
            },
        ]
    }

    pub fn extract_concepts(&self, description: &str) -> Vec<(CodeConcept, f64)> {
        let mut found = Vec::new();
        let desc_lower = description.to_lowercase();

        for mapping in &self.concept_mappings {
            for keyword in &mapping.keywords {
                if desc_lower.contains(keyword) {
                    found.push((mapping.concept.clone(), mapping.confidence));
                    break;
                }
            }
        }

        found
    }
}
```

#### 1.5.4 Smart Code Context Extraction

```rust
// src-tauri/src/nlp/code_context.rs

/// Extracts relevant code context based on natural language query.
pub struct CodeContextExtractor {
    analyzer: CodeAnalyzer,
}

impl CodeContextExtractor {
    pub async fn extract_relevant_context(
        &self,
        query: &str,
        file_path: &str,
        project: &ProjectContext
    ) -> Result<ExtractedContext, ExtractionError> {
        // 1. Parse the query to understand what's being asked
        let query_analysis = self.analyze_query(query)?;

        // 2. Find relevant files based on query
        let relevant_files = self.find_relevant_files(&query_analysis, project)?;

        // 3. Extract specific code sections from files
        let code_sections = self.extract_code_sections(&query_analysis, &relevant_files).await?;

        // 4. Build semantic context
        let semantic_context = self.build_semantic_context(query, &code_sections).await?;

        Ok(ExtractedContext {
            files: relevant_files,
            sections: code_sections,
            semantic_summary: semantic_context,
        })
    }

    async fn extract_code_sections(
        &self,
        analysis: &QueryAnalysis,
        files: &[FileInfo]
    ) -> Result<Vec<CodeSection>, ExtractionError> {
        let mut sections = Vec::new();

        for file in files {
            let code = std::fs::read_to_string(&file.path)?;
            let understanding = self.analyzer.analyze(&code, file.language)?;

            // Match sections based on query analysis
            for function in &understanding.structure.functions {
                if self.matches_query(&analysis, function) {
                    sections.push(CodeSection {
                        file_path: file.path.clone(),
                        name: function.name.clone(),
                        code: self.extract_range(&code, function.line_range),
                        kind: CodeSectionKind::Function,
                    });
                }
            }
        }

        Ok(sections)
    }

    async fn build_semantic_context(&self, query: &str, sections: &[CodeSection]) -> Result<String, ExtractionError> {
        // Use Claude to build semantic summary
        let prompt = format!(
            "Given this query: '{}'\n\nAnd these code sections:\n{}\n\nProvide a brief semantic context of how these relate to the query.",
            query,
            sections.iter()
                .map(|s| format!("{}:\n{}", s.name, s.code))
                .collect::<Vec<_>>()
                .join("\n\n")
        );

        let client = ClaudeClient::new();
        let response = client.complete(
            "You are a code context analyzer. Provide brief, technical summaries.",
            &prompt
        ).await?;

        Ok(response)
    }
}

#[derive(Debug, Clone)]
pub struct ExtractedContext {
    pub files: Vec<FileInfo>,
    pub sections: Vec<CodeSection>,
    pub semantic_summary: String,
}

#[derive(Debug, Clone)]
pub struct CodeSection {
    pub file_path: String,
    pub name: String,
    pub code: String,
    pub kind: CodeSectionKind,
}

#[derive(Debug, Clone)]
pub enum CodeSectionKind {
    Function,
    Class,
    Interface,
    Block,
}
```

#### 1.5.5 Frontend Code Context Display

```svelte
<!-- src-ui/src/lib/components/chat/CodeContextIndicator.svelte -->
<script lang="ts">
  import styles from './CodeContextIndicator.module.css';
  import type { ExtractedContext } from '$lib/types/nlp';

  interface Props {
    context: ExtractedContext;
  }

  let { context }: Props = $props();

  let expanded = $state(false);
</script>

<div class={styles.container}>
  <button
    class={styles.toggle}
    onclick={() => expanded = !expanded}
  >
    <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
      <path d="M14 2H6c-1.1 0-1.99.9-1.99 2L4 20c0 1.1.89 2 1.99 2H18c1.1 0 2-.9 2-2V8l-6-6zm2 16H8v-2h8v2zm0-4H8v-2h8v2zm-3-5V3.5L18.5 9H13z" />
    </svg>
    <span>{context.files.length} files, {context.sections.length} sections</span>
    <svg class={[styles.chevron, expanded && styles.open].filter(Boolean).join('')}
         width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
      <path d="M7.41 8.59L12 13.17l4.59-4.58L18 10l-6 6-6-6 1.41-1.41z" />
    </svg>
  </button>

  {#if expanded}
    <div class={styles.details}>
      {#if context.semantic_summary}
        <div class={styles.summary}>
          <h4>Context Summary</h4>
          <p>{context.semantic_summary}</p>
        </div>
      {/if}

      <div class={styles.sections}>
        <h4>Referenced Code</h4>
        {#each context.sections as section}
          <div class={styles.section}>
            <span class={styles.name}>{section.name}</span>
            <span class={styles.file}>{section.file_path}</span>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>
```

---

## Implementation Phases

### Phase 1: Foundation (Week 1-2)

**Backend Setup**
- [ ] Create NLP module structure in `src-tauri/src/nlp/`
- [ ] Implement intent taxonomy and basic types
- [ ] Set up Claude client integration
- [ ] Create SQLite schema for context storage
- [ ] Add dependencies: `tokio`, `rusqlite`, `serde`, `regex`

**Frontend Setup**
- [ ] Create NLP types in `src-ui/src/lib/types/nlp.ts`
- [ ] Set up base UI components for intent display

### Phase 2: Intent Recognition (Week 3-4)

- [ ] Implement `IntentPatterns` for regex-based matching
- [ ] Build `IntentClassifier` with Claude integration
- [ ] Create intent classification API endpoints
- [ ] Implement `IntentIndicator` Svelte component
- [ ] Add intent feedback mechanism (user corrections)

### Phase 3: Context Preservation (Week 5-6)

- [ ] Implement `ContextManager` with SQLite backend
- [ ] Build context compression using AI summarization
- [ ] Create semantic search with embeddings (optional)
- [ ] Implement context window management
- [ ] Build `ContextIndicator` UI component
- [ ] Add context pruning policies

### Phase 4: Multi-Intent Detection (Week 7)

- [ ] Implement `MultiIntentDetector`
- [ ] Build execution planner for intent orchestration
- [ ] Create dependency graph for intent relationships
- [ ] Implement `MultiIntentIndicator` UI
- [ ] Add intent preview before execution

### Phase 5: Ambiguity Resolution (Week 8)

- [ ] Implement `AmbiguityResolver` with symbol resolution
- [ ] Build clarification question generator
- [ ] Create suggestion system based on context
- [ ] Implement `ClarificationPrompt` UI component
- [ ] Add learning from user clarifications

### Phase 6: Code Understanding (Week 9-10)

- [ ] Integrate tree-sitter for code parsing
- [ ] Implement `CodeAnalyzer` for supported languages
- [ ] Build pattern detection system
- [ ] Create `NLToCodeMapper` for concept extraction
- [ ] Implement `CodeContextExtractor`
- [ ] Build `CodeContextIndicator` UI

### Phase 7: Integration & Testing (Week 11)

- [ ] Connect all NLP components through Tauri IPC
- [ ] Implement comprehensive error handling
- [ ] Add telemetry for intent accuracy metrics
- [ ] Create end-to-end tests for each feature
- [ ] Performance testing and optimization

### Phase 8: Polish & Documentation (Week 12)

- [ ] Accessibility audit (WCAG 2.1 AA)
- [ ] Visual polish of all UI components
- [ ] Write API documentation
- [ ] Create user-facing documentation
- [ ] Add examples and tutorials

---

## Dependencies

### Rust Dependencies

```toml
# src-tauri/Cargo.toml

[dependencies]
# Async runtime
tokio = { version = "1.35", features = ["full"] }

# Database
rusqlite = { version = "0.30", features = ["bundled"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# HTTP client for Claude API
reqwest = { version = "0.11", features = ["json"] }

# Regex for pattern matching
regex = "1.10"

# Code parsing
tree-sitter = "0.20"
tree-sitter-rust = "0.20"
tree-sitter-typescript = "0.20"
tree-sitter-javascript = "0.20"
tree-sitter-python = "0.20"

# Date/time
chrono = { version = "0.4", features = ["serde"] }

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Embeddings (optional for semantic search)
candle-core = "0.3"  # For local embeddings
candle-transformers = "0.3"
```

### NPM Dependencies

```json
{
  "dependencies": {
    "svelte": "^5.0.0",
    "@tauri-apps/api": "^2.0.0"
  }
}
```

---

## Tauri IPC Commands

```rust
// src-tauri/src/commands/nlp.rs

use crate::nlp::{
    classifier::IntentClassifier,
    context::ContextManager,
    multi_intent::MultiIntentDetector,
    ambiguity::AmbiguityResolver,
    code_understanding::CodeAnalyzer,
};

/// Classify intent from user input.
#[tauri::command]
async fn classify_intent(
    input: String,
    conversation_id: String,
    state: tauri::State<'_, AppState>
) -> Result<IntentClassification, String> {
    state.intent_classifier
        .classify(&input, &state.context_manager.get_context(&conversation_id).await?)
        .await
        .map_err(|e| e.to_string())
}

/// Get context for a conversation.
#[tauri::command]
async fn get_conversation_context(
    conversation_id: String,
    state: tauri::State<'_, AppState>
) -> Result<ContextWindow, String> {
    state.context_manager
        .load_context(&conversation_id)
        .await
        .map_err(|e| e.to_string())
}

/// Detect multiple intents in input.
#[tauri::command]
async fn detect_multi_intent(
    input: String,
    state: tauri::State<'_, AppState>
) -> Result<MultiIntentResult, String> {
    state.multi_detector
        .detect(&input)
        .await
        .map_err(|e| e.to_string())
}

/// Detect and resolve ambiguities.
#[tauri::command]
async fn detect_ambiguities(
    input: String,
    conversation_id: String,
    state: tauri::State<'_, AppState>
) -> Result<Vec<Ambiguity>, String> {
    let context = state.context_manager.load_context(&conversation_id).await?;
    state.ambiguity_resolver
        .detect_ambiguities(&input, &context)
        .await
        .map_err(|e| e.to_string())
}

/// Analyze code for understanding.
#[tauri::command]
async fn analyze_code(
    code: String,
    language: ProgrammingLanguage,
    state: tauri::State<'_, AppState>
) -> Result<CodeUnderstanding, String> {
    state.code_analyzer
        .analyze(&code, language)
        .map_err(|e| e.to_string())
}

/// Extract relevant code context.
#[tauri::command]
async fn extract_code_context(
    query: String,
    file_path: String,
    state: tauri::State<'_, AppState>
) -> Result<ExtractedContext, String> {
    state.code_context_extractor
        .extract_relevant_context(&query, &file_path, &state.project_context)
        .await
        .map_err(|e| e.to_string())
}
```

---

## File Structure

```
src-tauri/src/
├── nlp/
│   ├── mod.rs                    # NLP module exports
│   ├── intent.rs                 # Intent types and taxonomy
│   ├── classifier.rs             # Intent classification engine
│   ├── patterns.rs               # Pattern-based matching
│   ├── context.rs                # Context data models
│   ├── manager.rs                # Context manager
│   ├── multi_intent.rs           # Multi-intent types
│   ├── multi_detector.rs         # Multi-intent detection
│   ├── execution_planner.rs      # Intent execution planning
│   ├── ambiguity.rs              # Ambiguity types
│   ├── ambiguity_resolver.rs     # Ambiguity resolution
│   ├── code_understanding.rs     # Code understanding types
│   ├── code_analyzer.rs          # Code analysis engine
│   ├── nl_to_code.rs             # NL to code mapping
│   └── code_context.rs           # Code context extraction
├── storage/
│   ├── mod.rs
│   └── context_store.rs          # SQLite context storage
├── code_analysis/
│   ├── mod.rs
│   └── symbol_resolver.rs        # Symbol resolution
├── ai/
│   ├── mod.rs
│   └── claude.rs                 # Claude API client
└── commands/
    ├── mod.rs
    └── nlp.rs                    # NLP Tauri commands

src-ui/src/lib/
├── types/
│   └── nlp.ts                    # NLP type definitions
└── components/
    └── chat/
        ├── input/
        │   ├── IntentIndicator.svelte
        │   ├── IntentIndicator.module.css
        │   └── CommandPalette.svelte
        ├── ContextIndicator.svelte
        ├── ContextIndicator.module.css
        ├── MultiIntentIndicator.svelte
        ├── MultiIntentIndicator.module.css
        ├── ClarificationPrompt.svelte
        ├── ClarificationPrompt.module.css
        ├── CodeContextIndicator.svelte
        └── CodeContextIndicator.module.css
```

---

## API Design

### Intent Classification Flow

```
User Input
    ↓
Frontend (detect typing pause)
    ↓
Tauri IPC: classify_intent(input, conversation_id)
    ↓
Rust: IntentClassifier
    ├─→ Pattern matching (fast path)
    │   └─→ Return if high confidence
    └─→ Claude API classification
        └─→ IntentClassification
    ↓
Frontend: Update IntentIndicator UI
```

### Context Management Flow

```
User sends message
    ↓
Tauri IPC: add_message(conversation_id, message)
    ↓
Rust: ContextManager
    ├─→ Add to context window
    ├─→ Estimate tokens
    ├─→ Compress if needed (Claude summarization)
    └─→ Save to SQLite
    ↓
Claude API call with context
    ↓
Response + Update context
```

### Multi-Intent Execution Flow

```
User input with multiple intents
    ↓
Tauri IPC: detect_multi_intent(input)
    ↓
Rust: MultiIntentDetector
    ├─→ Split input
    ├─→ Classify each part
    ├─→ Determine relationships
    └─→ Create execution plan
    ↓
Frontend: Show MultiIntentIndicator
    ↓
User confirms execution
    ↓
Execute intents in order
    ├─→ Parallel intents execute together
    ├─→ Dependent intents wait for results
    └─→ Show progress for each
```

---

## Performance Considerations

1. **Intent Classification Caching**
   - Cache classifications for repeated inputs
   - TTL: 1 hour for pattern matches, 24 hours for AI classifications

2. **Context Window Management**
   - Compress context at 50% token capacity
   - Use semantic search to find relevant messages
   - Keep recent messages (last 20) always available

3. **Code Analysis Optimization**
   - Cache parsed AST trees
   - Incremental analysis for file changes
   - Lazy loading of symbol information

4. **Claude API Optimization**
   - Batch multiple intent classifications
   - Use smaller models (Haiku) for simple tasks
   - Cache common prompt responses

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_pattern_matching() {
        let patterns = IntentPatterns::new();
        let input = "generate a function for user authentication";
        let intent = patterns.match_input(input);
        assert_eq!(intent, Some(&Intent::GenerateCode));
    }

    #[test]
    fn test_context_compression() {
        // Test context window compression
    }

    #[test]
    fn test_multi_intent_detection() {
        // Test multi-intent parsing
    }
}
```

### Integration Tests

- Test full intent classification pipeline
- Test context persistence across conversations
- Test ambiguity resolution with real codebases
- Test code analysis with various languages

### End-to-End Tests

1. User: "Find bugs in auth.ts and write tests"
   - Expected: Detect 2 intents, show plan, execute sequentially

2. User: "Refactor the user service"
   - Expected: Detect ambiguity, ask which service

3. User: "Generate a React component for user profile"
   - Expected: Extract "React", "component", "user profile" concepts

---

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum NlpError {
    #[error("Claude API error: {0}")]
    ClaudeApi(String),

    #[error("Context not found: {0}")]
    ContextNotFound(String),

    #[error("Ambiguity resolution failed: {0}")]
    AmbiguityResolution(String),

    #[error("Code analysis failed: {0}")]
    CodeAnalysis(String),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

---

## Status: Ready for Implementation

This plan provides a comprehensive roadmap for implementing advanced NLP capabilities in the Coder AI desktop application. All components follow the project's architecture guidelines and integrate seamlessly with the existing codebase.

The implementation is organized into 8 phases spanning 12 weeks, with clear dependencies and deliverables for each phase.

Would you like me to begin implementing any specific phase or component from this plan?
