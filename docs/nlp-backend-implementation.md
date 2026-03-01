# NLP Backend Implementation Plan (Detailed)

> **Note**: This document provides the detailed Rust backend implementation for the NLP features outlined in [nlp-implementation-plan.md](./nlp-implementation-plan.md).

---

## Architecture Overview

```
src-tauri/src/
├── nlp/
│   ├── mod.rs                    # Module exports
│   ├── intent.rs                 # Intent types & taxonomy
│   ├── classifier.rs             # Intent classification engine
│   ├── patterns.rs               # Regex pattern matching
│   ├── context.rs                # Context data models
│   ├── context_manager.rs        # Context lifecycle management
│   ├── multi_intent.rs           # Multi-intent types
│   ├── multi_detector.rs         # Multi-intent detection
│   ├── execution_planner.rs      # Intent execution planning
│   ├── ambiguity.rs              # Ambiguity types
│   ├── ambiguity_resolver.rs     # Ambiguity resolution
│   ├── code_understanding.rs     # Code analysis types
│   ├── code_analyzer.rs          # tree-sitter code analysis
│   └── nl_to_code.rs             # Natural language → code mapping
├── storage/
│   ├── mod.rs
│   └── context_store.rs          # SQLite persistence
├── ai/
│   ├── mod.rs
│   └── claude.rs                 # Claude API client
└── commands/
    ├── mod.rs
    └── nlp.rs                    # Tauri IPC commands
```

---

## Task 1.1: Intent Recognition

### 1.1.1 Intent Taxonomy (`intent.rs`)

```rust
/// Developer command intents
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    // Code generation
    GenerateCode,
    GenerateFunction,
    GenerateClass,
    GenerateTest,
    RefactorCode,
    OptimizeCode,
    
    // Code understanding
    ExplainCode,
    AnalyzeCode,
    FindBugs,
    DocumentCode,
    
    // Navigation
    FindFile,
    FindSymbol,
    GotoDefinition,
    FindReferences,
    
    // Project management
    RunTests,
    BuildProject,
    FormatCode,
    LintCode,
    
    // Git operations
    GitStatus,
    GitCommit,
    GitDiff,
    GitLog,
    
    // Conversation
    StartNewThread,
    SwitchContext,
    ClearHistory,
    
    // Fallback
    Unknown,
}

/// Classification result with confidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentClassification {
    pub intent: Intent,
    pub confidence: f64,
    pub alternative_intents: Vec<(Intent, f64)>,
}

/// Extracted parameters from input
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IntentParams {
    pub language: Option<String>,
    pub file_path: Option<String>,
    pub symbol_name: Option<String>,
    pub code_snippet: Option<String>,
    pub description: Option<String>,
}
```

### 1.1.2 Pattern Matching (`patterns.rs`)

Fast-path classification using regex before calling AI:

```rust
use regex::Regex;
use crate::nlp::intent::Intent;

pub struct IntentPatterns {
    patterns: Vec<(Intent, Vec<Regex>)>,
}

impl IntentPatterns {
    pub fn new() -> Self {
        Self {
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
                ]),
                (Intent::GitCommit, vec![
                    Regex::new(r"(?i)^git\s+commit").unwrap(),
                    Regex::new(r"(?i)^commit\s+(these\s+)?changes").unwrap(),
                ]),
                // ... more patterns
            ],
        }
    }

    pub fn match_input(&self, input: &str) -> Option<&Intent> {
        for (intent, patterns) in &self.patterns {
            if patterns.iter().any(|p| p.is_match(input)) {
                return Some(intent);
            }
        }
        None
    }
}
```

### 1.1.3 Intent Classifier (`classifier.rs`)

```rust
use crate::nlp::intent::{Intent, IntentClassification, IntentParams};
use crate::nlp::patterns::IntentPatterns;
use crate::nlp::context::ConversationContext;
use crate::ai::claude::ClaudeClient;

pub struct IntentClassifier {
    claude: ClaudeClient,
    patterns: IntentPatterns,
}

impl IntentClassifier {
    pub fn new(claude: ClaudeClient) -> Self {
        Self {
            claude,
            patterns: IntentPatterns::new(),
        }
    }

    /// Classify intent from user input
    pub async fn classify(
        &self,
        input: &str,
        context: &ConversationContext,
    ) -> Result<IntentClassification, ClassifierError> {
        // Fast path: pattern matching
        if let Some(intent) = self.patterns.match_input(input) {
            return Ok(IntentClassification {
                intent: intent.clone(),
                confidence: 0.95,
                alternative_intents: vec![],
            });
        }

        // Slow path: AI classification
        self.classify_with_ai(input, context).await
    }

    async fn classify_with_ai(
        &self,
        input: &str,
        context: &ConversationContext,
    ) -> Result<IntentClassification, ClassifierError> {
        let system_prompt = self.build_prompt(context);
        let response = self.claude
            .complete(&system_prompt, input)
            .await?;

        self.parse_response(&response)
    }

    fn build_prompt(&self, context: &ConversationContext) -> String {
        format!(
            r#"You are an intent classifier for a developer AI assistant.
Classify the user's input into exactly one of these intents:
- generate_code, generate_function, generate_class, generate_test
- refactor_code, optimize_code
- explain_code, analyze_code, find_bugs, document_code
- find_file, find_symbol, goto_definition, find_references
- run_tests, build_project, format_code, lint_code
- git_status, git_commit, git_diff, git_log
- start_new_thread, switch_context, clear_history
- unknown

Current context: {:?}

Return JSON: {{"intent": "...", "confidence": 0.0-1.0, "alternatives": [[intent, score], ...]}}"#,
            context.summary()
        )
    }

    fn parse_response(&self, response: &str) -> Result<IntentClassification, ClassifierError> {
        serde_json::from_str(response)
            .map_err(|e| ClassifierError::ParseError(e.to_string()))
    }
}
```

---

## Task 1.2: Context Preservation

### 1.2.1 Context Data Model (`context.rs`)

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A conversation context window
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

/// A single message in context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMessage {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub tokens: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// Metadata tracking current state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextMetadata {
    pub current_file: Option<FileInfo>,
    pub active_symbols: Vec<SymbolReference>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolReference {
    pub name: String,
    pub kind: SymbolKind,
    pub file_path: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Class,
    Variable,
    Interface,
}

/// Simplified context for AI prompts
#[derive(Debug, Clone)]
pub struct ConversationContext {
    pub recent_messages: Vec<ContextMessage>,
    pub metadata: ContextMetadata,
    pub summary: Option<String>,
}

impl ConversationContext {
    pub fn summary(&self) -> String {
        format!(
            "File: {:?}, Symbols: {:?}, Recent: {} messages",
            self.metadata.current_file.as_ref().map(|f| &f.path),
            self.metadata.active_symbols.iter().map(|s| &s.name).collect::<Vec<_>>(),
            self.recent_messages.len()
        )
    }
}
```

### 1.2.2 Context Manager (`context_manager.rs`)

```rust
use crate::nlp::context::{ContextWindow, ContextMessage, ConversationContext, ContextMetadata};
use crate::storage::context_store::ContextStore;
use crate::ai::claude::ClaudeClient;

const MAX_TOKENS: usize = 100_000;
const COMPRESSION_THRESHOLD: usize = 50_000;

pub struct ContextManager {
    store: ContextStore,
    claude: ClaudeClient,
}

impl ContextManager {
    pub fn new(store: ContextStore, claude: ClaudeClient) -> Self {
        Self { store, claude }
    }

    /// Add a message to the conversation context
    pub async fn add_message(
        &self,
        conversation_id: &str,
        message: ContextMessage,
    ) -> Result<(), ContextError> {
        let mut window = self.store.load(conversation_id).await?;
        
        window.messages.push(message);
        window.updated_at = Utc::now();

        // Check for compression
        if self.estimate_tokens(&window) > COMPRESSION_THRESHOLD {
            window = self.compress(window).await?;
        }

        self.store.save(&window).await
    }

    /// Get context for AI prompt
    pub async fn get_context(
        &self,
        conversation_id: &str,
    ) -> Result<ConversationContext, ContextError> {
        let window = self.store.load(conversation_id).await?;
        
        Ok(ConversationContext {
            recent_messages: window.messages.iter().rev().take(20).rev().cloned().collect(),
            metadata: window.metadata.clone(),
            summary: window.metadata.summary.clone(),
        })
    }

    /// Update current file/symbols
    pub async fn update_metadata(
        &self,
        conversation_id: &str,
        metadata: ContextMetadata,
    ) -> Result<(), ContextError> {
        let mut window = self.store.load(conversation_id).await?;
        window.metadata = metadata;
        window.updated_at = Utc::now();
        self.store.save(&window).await
    }

    fn estimate_tokens(&self, window: &ContextWindow) -> usize {
        // Rough estimate: ~4 chars per token
        window.messages
            .iter()
            .map(|m| m.content.len() / 4)
            .sum()
    }

    async fn compress(
        &self,
        mut window: ContextWindow,
    ) -> Result<ContextWindow, ContextError> {
        let half = window.messages.len() / 2;
        let to_compress: Vec<_> = window.messages.drain(0..half).collect();
        
        let summary = self.generate_summary(&to_compress).await?;
        window.metadata.summary = Some(summary);

        Ok(window)
    }

    async fn generate_summary(
        &self,
        messages: &[ContextMessage],
    ) -> Result<String, ContextError> {
        let content: String = messages
            .iter()
            .map(|m| format!("{:?}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Summarize this conversation, preserving key technical details:\n\n{}",
            content
        );

        self.claude
            .complete("You are a conversation summarizer.", &prompt)
            .await
            .map_err(ContextError::AiError)
    }
}
```

### 1.2.3 SQLite Store (`context_store.rs`)

```rust
use rusqlite::{Connection, params, OptionalExtension};
use crate::nlp::context::{ContextWindow, ContextMessage, ContextMetadata, MessageRole};

pub struct ContextStore {
    conn: std::sync::Mutex<Connection>,
}

impl ContextStore {
    pub fn new(db_path: &str) -> Result<Self, StoreError> {
        let conn = Connection::open(db_path)?;
        Self::init_schema(&conn)?;
        
        Ok(Self {
            conn: std::sync::Mutex::new(conn),
        })
    }

    fn init_schema(conn: &Connection) -> Result<(), StoreError> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS context_windows (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL UNIQUE,
                thread_id TEXT,
                metadata TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS context_messages (
                id TEXT PRIMARY KEY,
                context_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                tokens INTEGER,
                FOREIGN KEY (context_id) REFERENCES context_windows(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_messages_context 
            ON context_messages(context_id);
            "#,
        )?;
        Ok(())
    }

    pub async fn load(&self, conversation_id: &str) -> Result<ContextWindow, StoreError> {
        let conn = self.conn.lock().map_err(|_| StoreError::LockError)?;
        
        // Load window
        let window: Option<(String, Option<String>, String, String, String)> = conn
            .query_row(
                "SELECT id, thread_id, metadata, created_at, updated_at 
                 FROM context_windows WHERE conversation_id = ?1",
                params![conversation_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .optional()?;

        let (id, thread_id, metadata_json, created_at, updated_at) = 
            window.ok_or(StoreError::NotFound)?;

        let metadata: ContextMetadata = serde_json::from_str(&metadata_json)?;
        
        // Load messages
        let mut stmt = conn.prepare(
            "SELECT id, role, content, timestamp, tokens 
             FROM context_messages WHERE context_id = ?1 
             ORDER BY timestamp ASC"
        )?;
        
        let messages = stmt
            .query_map(params![&id], |row| {
                Ok(ContextMessage {
                    id: row.get(0)?,
                    role: match row.get::<_, String>(1)?.as_str() {
                        "user" => MessageRole::User,
                        "assistant" => MessageRole::Assistant,
                        "system" => MessageRole::System,
                        _ => MessageRole::User,
                    },
                    content: row.get(2)?,
                    timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    tokens: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ContextWindow {
            id,
            conversation_id: conversation_id.to_string(),
            thread_id,
            messages,
            metadata,
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))?
                .unwrap_or_else(chrono::Utc::now),
            updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))?
                .unwrap_or_else(chrono::Utc::now),
        })
    }

    pub async fn save(&self, window: &ContextWindow) -> Result<(), StoreError> {
        let conn = self.conn.lock().map_err(|_| StoreError::LockError)?;
        
        let metadata_json = serde_json::to_string(&window.metadata)?;
        
        // Upsert window
        conn.execute(
            r#"INSERT OR REPLACE INTO context_windows 
               (id, conversation_id, thread_id, metadata, created_at, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
            params![
                &window.id,
                &window.conversation_id,
                &window.thread_id,
                &metadata_json,
                &window.created_at.to_rfc3339(),
                &window.updated_at.to_rfc3339(),
            ],
        )?;

        // Delete old messages and insert new
        conn.execute("DELETE FROM context_messages WHERE context_id = ?1", params![&window.id])?;
        
        for msg in &window.messages {
            conn.execute(
                "INSERT INTO context_messages (id, context_id, role, content, timestamp, tokens)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    &msg.id,
                    &window.id,
                    match msg.role {
                        MessageRole::User => "user",
                        MessageRole::Assistant => "assistant",
                        MessageRole::System => "system",
                    },
                    &msg.content,
                    &msg.timestamp.to_rfc3339(),
                    msg.tokens,
                ],
            )?;
        }

        Ok(())
    }

    pub async fn create(&self, conversation_id: &str) -> Result<ContextWindow, StoreError> {
        let window = ContextWindow {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: conversation_id.to_string(),
            thread_id: None,
            messages: vec![],
            metadata: ContextMetadata::default(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        
        self.save(&window).await?;
        Ok(window)
    }
}
```

---

## Task 1.3: Multi-Intent Detection

### 1.3.1 Types (`multi_intent.rs`)

```rust
use serde::{Deserialize, Serialize};
use crate::nlp::intent::{Intent, IntentParams};

/// Detected intent with position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedIntent {
    pub intent: Intent,
    pub params: IntentParams,
    pub span: TextSpan,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSpan {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// Result of multi-intent detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiIntentResult {
    pub intents: Vec<DetectedIntent>,
    pub relationships: Vec<IntentRelationship>,
    pub execution_order: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentRelationship {
    Sequential,
    Parallel,
    Dependent,
    Exclusive,
}
```

### 1.3.2 Detector (`multi_detector.rs`)

```rust
use crate::nlp::multi_intent::{DetectedIntent, MultiIntentResult, IntentRelationship, TextSpan};
use crate::nlp::classifier::IntentClassifier;
use crate::nlp::context::ConversationContext;
use crate::nlp::intent::IntentParams;
use crate::ai::claude::ClaudeClient;

pub struct MultiIntentDetector {
    claude: ClaudeClient,
    classifier: IntentClassifier,
}

impl MultiIntentDetector {
    pub fn new(claude: ClaudeClient, classifier: IntentClassifier) -> Self {
        Self { claude, classifier }
    }

    pub async fn detect(
        &self,
        input: &str,
        context: &ConversationContext,
    ) -> Result<MultiIntentResult, DetectorError> {
        // Check for conjunctions
        if !self.has_conjunctions(input) {
            // Single intent
            let classification = self.classifier.classify(input, context).await?;
            return Ok(MultiIntentResult {
                intents: vec![DetectedIntent {
                    intent: classification.intent,
                    params: IntentParams::default(),
                    span: TextSpan { start: 0, end: input.len(), text: input.to_string() },
                    confidence: classification.confidence,
                }],
                relationships: vec![],
                execution_order: vec![0],
            });
        }

        // Multi-intent: use AI
        self.detect_with_ai(input, context).await
    }

    fn has_conjunctions(&self, input: &str) -> bool {
        let lower = input.to_lowercase();
        [" and ", " then ", " after ", " before ", " while "]
            .iter()
            .any(|c| lower.contains(c))
    }

    async fn detect_with_ai(
        &self,
        input: &str,
        context: &ConversationContext,
    ) -> Result<MultiIntentResult, DetectorError> {
        let prompt = format!(
            r#"Analyze this developer query for multiple intents: '{}'

Return JSON:
{{
  "intents": [
    {{"intent": "...", "span": {{"start": n, "end": n, "text": "..."}}, "confidence": 0.0-1.0}}
  ],
  "relationships": ["sequential" | "parallel" | "dependent" | "exclusive"],
  "execution_order": [0, 1, ...]
}}

Context: {}"#,
            input,
            context.summary()
        );

        let response = self.claude
            .complete("You are a multi-intent detector.", &prompt)
            .await?;

        self.parse_response(&response)
    }

    fn parse_response(&self, json: &str) -> Result<MultiIntentResult, DetectorError> {
        serde_json::from_str(json)
            .map_err(|e| DetectorError::ParseError(e.to_string()))
    }
}
```

### 1.3.3 Execution Planner (`execution_planner.rs`)

```rust
use crate::nlp::multi_intent::{MultiIntentResult, IntentRelationship};

#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub levels: Vec<Vec<usize>>,
}

pub struct ExecutionPlanner;

impl ExecutionPlanner {
    pub fn create(result: &MultiIntentResult) -> ExecutionPlan {
        let mut levels = Vec::new();
        let mut completed = std::collections::HashSet::new();

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
                break; // Circular dependency
            }
            levels.push(level);
        }

        ExecutionPlan { levels }
    }

    fn can_execute(
        result: &MultiIntentResult,
        index: usize,
        completed: &std::collections::HashSet<usize>,
    ) -> bool {
        // Check if all dependencies are satisfied
        for (i, rel) in result.relationships.iter().enumerate() {
            if matches!(rel, IntentRelationship::Dependent | IntentRelationship::Sequential) {
                // If this intent depends on a previous one
                if i < index && !completed.contains(&i) {
                    return false;
                }
            }
        }
        true
    }
}
```

---

## Task 1.4: Ambiguity Resolution

### 1.4.1 Types (`ambiguity.rs`)

```rust
use serde::{Deserialize, Serialize};
use crate::nlp::multi_intent::TextSpan;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmbiguityType {
    MissingFile,
    MissingSymbol,
    AmbiguousReference,
    MultipleMatches,
    UnclearScope,
}

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
    Low,
    Medium,
    High,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmbiguitySuggestion {
    pub value: String,
    pub confidence: f64,
    pub reason: String,
}
```

### 1.4.2 Resolver (`ambiguity_resolver.rs`)

```rust
use crate::nlp::ambiguity::*;
use crate::nlp::context::ConversationContext;
use crate::nlp::multi_intent::TextSpan;
use regex::Regex;

pub struct AmbiguityResolver {
    file_pattern: Regex,
}

impl AmbiguityResolver {
    pub fn new() -> Self {
        Self {
            file_pattern: Regex::new(r"[a-zA-Z0-9_/\\]+\.[a-zA-Z0-9]+").unwrap(),
        }
    }

    pub async fn detect(
        &self,
        input: &str,
        context: &ConversationContext,
        project_files: &[String],
    ) -> Result<Vec<Ambiguity>, ResolverError> {
        let mut ambiguities = Vec::new();

        // Check file references
        for cap in self.file_pattern.find_iter(input) {
            let file_ref = cap.as_str();
            let matches: Vec<_> = project_files
                .iter()
                .filter(|f| f.ends_with(file_ref) || f.contains(file_ref))
                .collect();

            match matches.len() {
                0 => {
                    ambiguities.push(Ambiguity {
                        ambiguity_type: AmbiguityType::MissingFile,
                        span: TextSpan {
                            start: cap.start(),
                            end: cap.end(),
                            text: file_ref.to_string(),
                        },
                        severity: AmbiguitySeverity::High,
                        clarification: ClarificationRequest {
                            question: format!("File '{}' not found. Specify the correct path:", file_ref),
                            options: vec![],
                            allows_freeform: true,
                        },
                        suggestions: self.fuzzy_match_files(file_ref, project_files),
                    });
                }
                2.. => {
                    ambiguities.push(Ambiguity {
                        ambiguity_type: AmbiguityType::MultipleMatches,
                        span: TextSpan {
                            start: cap.start(),
                            end: cap.end(),
                            text: file_ref.to_string(),
                        },
                        severity: AmbiguitySeverity::Medium,
                        clarification: ClarificationRequest {
                            question: format!("Multiple files match '{}'. Which one?", file_ref),
                            options: matches.iter().take(5).map(|f| ClarificationOption {
                                value: (*f).clone(),
                                label: (*f).clone(),
                                description: None,
                            }).collect(),
                            allows_freeform: true,
                        },
                        suggestions: vec![],
                    });
                }
                _ => {}
            }
        }

        Ok(ambiguities)
    }

    fn fuzzy_match_files(&self, input: &str, files: &[String]) -> Vec<AmbiguitySuggestion> {
        // Simple fuzzy matching - could use fuzzy search library
        files
            .iter()
            .filter(|f| {
                let lower = f.to_lowercase();
                input.chars().all(|c| lower.contains(c.to_ascii_lowercase()))
            })
            .take(3)
            .map(|f| AmbiguitySuggestion {
                value: f.clone(),
                confidence: 0.7,
                reason: "fuzzy match".to_string(),
            })
            .collect()
    }
}
```

---

## Task 1.5: Code Understanding

### 1.5.1 Types (`code_understanding.rs`)

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeUnderstanding {
    pub language: ProgrammingLanguage,
    pub structure: CodeStructure,
    pub complexity: ComplexityMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProgrammingLanguage {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeStructure {
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
    pub imports: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
    pub line_start: usize,
    pub line_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    pub name: String,
    pub methods: Vec<FunctionInfo>,
    pub line_start: usize,
    pub line_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    pub cyclomatic: usize,
    pub lines: usize,
    pub nesting: usize,
}
```

### 1.5.2 Analyzer (`code_analyzer.rs`)

```rust
use tree_sitter::{Parser, Node};
use crate::nlp::code_understanding::*;

pub struct CodeAnalyzer {
    parsers: std::collections::HashMap<ProgrammingLanguage, Parser>,
}

impl CodeAnalyzer {
    pub fn new() -> Result<Self, AnalyzerError> {
        let mut parsers = std::collections::HashMap::new();
        
        // Initialize parsers (requires tree-sitter grammars)
        let mut rust_parser = Parser::new();
        rust_parser.set_language(&tree_sitter_rust::LANGUAGE.into())
            .map_err(|e| AnalyzerError::ParserInit(e.to_string()))?;
        parsers.insert(ProgrammingLanguage::Rust, rust_parser);

        let mut ts_parser = Parser::new();
        ts_parser.set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .map_err(|e| AnalyzerError::ParserInit(e.to_string()))?;
        parsers.insert(ProgrammingLanguage::TypeScript, ts_parser);

        Ok(Self { parsers })
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
            complexity: self.calculate_complexity(&root, code),
        })
    }

    fn extract_structure(&self, root: &Node, source: &str) -> Result<CodeStructure, AnalyzerError> {
        let mut functions = Vec::new();
        let mut classes = Vec::new();
        let mut imports = Vec::new();

        Self::traverse(root, &mut |node| {
            match node.kind() {
                "function_declaration" | "function_item" => {
                    if let Some(func) = Self::extract_function(node, source) {
                        functions.push(func);
                    }
                }
                "class_declaration" | "struct_item" | "impl_item" => {
                    if let Some(class) = Self::extract_class(node, source) {
                        classes.push(class);
                    }
                }
                "import_statement" | "use_declaration" => {
                    if let Some(text) = node.utf8_text(source.as_bytes()).ok() {
                        imports.push(text.to_string());
                    }
                }
                _ => {}
            }
        });

        Ok(CodeStructure { functions, classes, imports })
    }

    fn extract_function(node: Node, source: &str) -> Option<FunctionInfo> {
        let name = node.child_by_field_name("name")?
            .utf8_text(source.as_bytes())
            .ok()?
            .to_string();

        let line_start = node.start_position().row;
        let line_end = node.end_position().row;

        Some(FunctionInfo {
            name,
            parameters: vec![], // Extract from params node
            return_type: None,
            line_start,
            line_end,
        })
    }

    fn extract_class(node: Node, source: &str) -> Option<ClassInfo> {
        let name = node.child_by_field_name("name")?
            .utf8_text(source.as_bytes())
            .ok()?
            .to_string();

        Some(ClassInfo {
            name,
            methods: vec![],
            line_start: node.start_position().row,
            line_end: node.end_position().row,
        })
    }

    fn calculate_complexity(&self, root: &Node, source: &str) -> ComplexityMetrics {
        let mut cyclomatic = 1;
        let mut max_nesting = 0;

        Self::traverse(root, &mut |node| {
            match node.kind() {
                "if_statement" | "while_statement" | "for_statement" 
                | "match_statement" | "if_expression" => {
                    cyclomatic += 1;
                }
                _ => {}
            }
        });

        ComplexityMetrics {
            cyclomatic,
            lines: source.lines().count(),
            nesting: max_nesting,
        }
    }

    fn traverse<F>(node: &Node, f: &mut F)
    where
        F: FnMut(Node),
    {
        f(node.clone());
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                Self::traverse(&child, f);
            }
        }
    }
}
```

---

## Claude Client (`ai/claude.rs`)

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct ClaudeClient {
    client: Client,
    api_key: String,
    model: String,
}

#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ClaudeMessage>,
}

#[derive(Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
}

#[derive(Deserialize)]
struct ClaudeContent {
    text: String,
}

impl ClaudeClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: "claude-sonnet-4-20250514".to_string(),
        }
    }

    pub async fn complete(
        &self,
        system: &str,
        user: &str,
    ) -> Result<String, ClaudeError> {
        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 4096,
            messages: vec![
                ClaudeMessage {
                    role: "user".to_string(),
                    content: format!("{}\n\n{}", system, user),
                },
            ],
        };

        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?
            .json::<ClaudeResponse>()
            .await?;

        Ok(response.content.first()
            .map(|c| c.text.clone())
            .unwrap_or_default())
    }
}
```

---

## Tauri Commands (`commands/nlp.rs`)

```rust
use crate::nlp::*;
use crate::AppState;

#[tauri::command]
pub async fn classify_intent(
    input: String,
    conversation_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<IntentClassification, String> {
    let context = state.context_manager
        .get_context(&conversation_id)
        .await
        .map_err(|e| e.to_string())?;

    state.classifier
        .classify(&input, &context)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn detect_multi_intent(
    input: String,
    conversation_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<MultiIntentResult, String> {
    let context = state.context_manager
        .get_context(&conversation_id)
        .await
        .map_err(|e| e.to_string())?;

    state.multi_detector
        .detect(&input, &context)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_context_message(
    conversation_id: String,
    role: String,
    content: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let message = ContextMessage {
        id: uuid::Uuid::new_v4().to_string(),
        role: match role.as_str() {
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            _ => MessageRole::System,
        },
        content,
        timestamp: chrono::Utc::now(),
        tokens: None,
    };

    state.context_manager
        .add_message(&conversation_id, message)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn detect_ambiguities(
    input: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Ambiguity>, String> {
    let files = state.project_files.lock().map_err(|_| "Lock error")?;
    let context = ConversationContext::default();

    state.ambiguity_resolver
        .detect(&input, &context, &files)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn analyze_code(
    code: String,
    language: ProgrammingLanguage,
    state: tauri::State<'_, AppState>,
) -> Result<CodeUnderstanding, String> {
    state.code_analyzer
        .analyze(&code, language)
        .map_err(|e| e.to_string())
}
```

---

## Required Dependencies (Cargo.toml additions)

```toml
[dependencies]
# ... existing ...

# NLP
regex = "1"
chrono = { version = "0.4", features = ["serde"] }

# Database
rusqlite = { version = "0.31", features = ["bundled"] }

# HTTP
reqwest = { version = "0.12", features = ["json"] }

# Code parsing
tree-sitter = "0.22"
tree-sitter-rust = "0.21"
tree-sitter-typescript = "0.21"
tree-sitter-python = "0.21"
```

---

## Error Types

```rust
// In each module, define specific errors:

#[derive(Debug, thiserror::Error)]
pub enum ClassifierError {
    #[error("AI error: {0}")]
    AiError(#[from] crate::ai::claude::ClaudeError),
    #[error("Parse error: {0}")]
    ParseError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    #[error("Store error: {0}")]
    StoreError(#[from] crate::storage::context_store::StoreError),
    #[error("AI error: {0}")]
    AiError(#[from] crate::ai::claude::ClaudeError),
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Not found")]
    NotFound,
    #[error("Lock error")]
    LockError,
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum DetectorError {
    #[error("AI error: {0}")]
    AiError(#[from] crate::ai::claude::ClaudeError),
    #[error("Parse error: {0}")]
    ParseError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum AnalyzerError {
    #[error("Unsupported language")]
    UnsupportedLanguage,
    #[error("Parse error")]
    ParseError,
    #[error("Parser init failed: {0}")]
    ParserInit(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ClaudeError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API error: {0}")]
    Api(String),
}
```

---

## Implementation Order

| Phase | Files | Priority |
|-------|-------|----------|
| **1** | `ai/claude.rs` + `storage/context_store.rs` | Foundation |
| **2** | `intent.rs`, `patterns.rs`, `classifier.rs` | Task 1.1 |
| **3** | `context.rs`, `context_manager.rs` | Task 1.2 |
| **4** | `multi_intent.rs`, `multi_detector.rs`, `execution_planner.rs` | Task 1.3 |
| **5** | `ambiguity.rs`, `ambiguity_resolver.rs` | Task 1.4 |
| **6** | `code_understanding.rs`, `code_analyzer.rs`, `nl_to_code.rs` | Task 1.5 |
| **7** | `commands/nlp.rs` + wire up in `lib.rs` | Integration |
