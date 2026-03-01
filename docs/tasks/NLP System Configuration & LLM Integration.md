# Task: NLP System Configuration & LLM Integration

## Task Overview
Configure the Natural Language Processing system, manage API token authentication, and implement model selection capabilities for integration with real LLM providers.

## Task Description
Develop and implement the infrastructure for connecting to external LLM providers, managing authentication credentials, and providing model selection functionality within the FocusFlow application.

## Key Responsibilities

### 1. System Analysis & Planning
- **Analyze requirements**: Document necessary LLM provider integrations and authentication methods
- **Evaluate providers**: Research available LLM providers (OpenAI, Anthropic, Google, etc.)
- **Assess security needs**: Determine token management and storage requirements
- **Plan integration approach**: Design architecture for seamless LLM communication
- **Cost analysis**: Evaluate pricing models and usage limits

### 2. Manager Confirmation & Approval
- **Present findings**: Share analysis results with product manager
- **Discuss approach**: Confirm integration strategy and provider selection
- **Secure approval**: Get sign-off on technical implementation plan
- **Define success criteria**: Establish measurable goals for integration

### 3. API Token Management System
- **Create token storage**: Implement secure storage for API credentials
- **Develop management interface**: Create UI for adding, editing, and revoking tokens
- **Implement validation**: Add token validity checking and expiration handling
- **Security measures**: Ensure compliance with security best practices
- **User permissions**: Define access control for token management

### 4. Model Selection Framework
- **Provider integration**: Connect to selected LLM provider APIs
- **Model discovery**: Implement dynamic model listing and capabilities
- **Selection interface**: Create UI for choosing models based on task requirements
- **Configuration management**: Store and apply model-specific settings
- **Performance tracking**: Monitor and display model performance metrics

### 5. Real LLM Integration
- **API connection**: Establish secure connections to LLM provider endpoints
- **Request formatting**: Implement proper API request structures
- **Response handling**: Process and format LLM responses appropriately
- **Error management**: Handle API errors and provider limitations
- **Rate limiting**: Implement request throttling and retry mechanisms

### 6. Testing & Validation
- **Provider testing**: Verify connections with all integrated providers
- **Token validation**: Test authentication and token functionality
- **Model testing**: Validate model selection and switching
- **Performance testing**: Measure response times and reliability
- **User acceptance**: Conduct testing with real developer scenarios

## Success Criteria

### Must-Have (P0)
- ✅ Secure API token management system
- ✅ Connection to at least 2 major LLM providers
- ✅ Functional model selection interface
- ✅ Real-time LLM communication working
- ✅ Manager approval of implementation approach

### Should-Have (P1)
- ✅ Support for 3+ LLM providers
- ✅ Token expiration and renewal handling
- ✅ Model performance comparison features
- ✅ Error logging and diagnostics
- ✅ Documentation for administrators

### Could-Have (P2)
- ✅ Automated provider onboarding
- ✅ Cost estimation and usage tracking
- ✅ Model recommendation system
- ✅ Multi-account support
- ✅ Backup provider switching

## Dependencies
- Product manager approval for provider selection
- Access to LLM provider API documentation
- Security team review of token storage approach

## Timeline
- **Weeks 1-2**: Analysis, planning, and manager approval
- **Weeks 3-4**: Token management system development
- **Weeks 5-6**: LLM provider integration
- **Weeks 7-8**: Model selection framework implementation
- **Weeks 9-10**: Testing, validation, and deployment

## Risks & Mitigation
- **Provider API changes**: Regular monitoring and quick adaptation
- **Token security issues**: Implement robust encryption and access controls
- **Performance limitations**: Optimize requests and implement caching
- **Cost overruns**: Implement usage monitoring and alerts

This task establishes the foundation for AI-powered capabilities in FocusFlow by creating the essential infrastructure for connecting to and utilizing external LLM services.

---

## Impact on Existing Codebase

This section details the required changes to existing files to support multi-provider LLM integration.

### Files Requiring Modification

#### 1. `src-tauri/src/ai/mod.rs`
**Current:**
```rust
mod claude;
pub use claude::{ClaudeClient, ClaudeError};
```
**Impact:** HIGH - Must add provider abstraction
**Changes:**
- Add `mod provider;`, `mod openai;`, `mod models;`
- Re-export `LlmProvider` trait and `ProviderRegistry`
- Update to support dynamic provider loading

---

#### 2. `src-tauri/src/ai/claude.rs`
**Current:** Direct implementation with hardcoded model
```rust
pub struct ClaudeClient {
    client: Client,
    api_key: String,
    model: String,  // Hardcoded "claude-sonnet-4-20250514"
}
```
**Impact:** HIGH - Must implement provider trait
**Changes:**
- Implement `LlmProvider` trait
- Add `list_models()` method
- Add `validate_api_key()` method
- Accept model as parameter in `complete()` instead of struct field
- Update error handling to use common `ProviderError`

---

#### 3. `src-tauri/src/commands/nlp.rs`
**Current:** Hardcoded API key
```rust
pub fn new(api_key: &str, db_path: &str) -> Result<Self, String> {
    let claude = Arc::new(ClaudeClient::new(api_key));
    // ...
}

pub fn new_dev(db_path: &str) -> Result<Self, String> {
    Self::new("dev-api-key", db_path)  // ← PROBLEM
}
```
**Impact:** HIGH - Must use config system
**Changes:**
- Replace `Arc<ClaudeClient>` with `ProviderRegistry`
- Load API keys from `SecretsStore` (keyring)
- Remove hardcoded `"dev-api-key"`
- Add `ConfigState` parameter or access
- Update all methods to use provider registry

**Affected Struct:**
```rust
pub struct NlpState {
    pub classifier: IntentClassifier,       // Uses ClaudeClient internally
    pub context_manager: ContextManager,    // Uses ClaudeClient internally
    pub multi_detector: MultiIntentDetector,// Uses ClaudeClient internally
    pub ambiguity_resolver: AmbiguityResolver,
    pub code_analyzer: CodeAnalyzer,
    pub project_files: Arc<Mutex<Vec<String>>>,
}
```
**Change to:**
```rust
pub struct NlpState {
    pub provider_registry: Arc<ProviderRegistry>,  // NEW
    pub classifier: IntentClassifier,
    pub context_manager: ContextManager,
    pub multi_detector: MultiIntentDetector,
    pub ambiguity_resolver: AmbiguityResolver,
    pub code_analyzer: CodeAnalyzer,
    pub project_files: Arc<Mutex<Vec<String>>>,
}
```

---

#### 4. `src-tauri/src/nlp/classifier.rs`
**Current:** Tightly coupled to ClaudeClient
```rust
pub struct IntentClassifier {
    claude: Arc<ClaudeClient>,  // ← Direct dependency
    patterns: IntentPatterns,
}
```
**Impact:** MEDIUM - Abstract to use provider trait
**Changes:**
- Change `claude: Arc<ClaudeClient>` to `provider: Arc<dyn LlmProvider>`
- Update error type from `ClaudeError` to `ProviderError`
- No changes to public API, only internal implementation

---

#### 5. `src-tauri/src/nlp/context_manager.rs`
**Current:** Tightly coupled to ClaudeClient
```rust
pub struct ContextManager {
    store: ContextStore,
    claude: Arc<ClaudeClient>,  // ← Direct dependency
}
```
**Impact:** MEDIUM - Abstract to use provider trait
**Changes:**
- Change `claude: Arc<ClaudeClient>` to `provider: Arc<dyn LlmProvider>`
- Update `compress()` and `generate_summary()` to use trait
- Update error handling

---

#### 6. `src-tauri/src/nlp/multi_detector.rs`
**Current:** Tightly coupled to ClaudeClient
```rust
pub struct MultiIntentDetector {
    claude: Arc<ClaudeClient>,  // ← Direct dependency
    patterns: IntentPatterns,
    conjunctions: Vec<&'static str>,
}
```
**Impact:** MEDIUM - Abstract to use provider trait
**Changes:**
- Change `claude: Arc<ClaudeClient>` to `provider: Arc<dyn LlmProvider>`
- Update error type from `ClaudeError` to `ProviderError`

---

#### 7. `src-tauri/src/lib.rs`
**Current:** Creates NlpState with hardcoded key
```rust
let nlp_state = NlpState::new_dev("coder_context.db")
    .expect("Failed to initialize NLP state");
```
**Impact:** HIGH - Must load config before init
**Changes:**
- Initialize config module first
- Load provider settings from storage
- Initialize provider registry with configured providers
- Pass registry to NlpState
- Add config commands to invoke handler

**New initialization flow:**
```rust
// 1. Load config
let config_state = ConfigState::new("coder_config.db")?;

// 2. Initialize provider registry
let provider_registry = Arc::new(ProviderRegistry::new());

// 3. Configure providers from stored settings
for provider_config in config_state.load_providers()? {
    provider_registry.register(provider_config)?;
}

// 4. Create NLP state with registry
let nlp_state = NlpState::new(provider_registry, "coder_context.db")?;
```

---

#### 8. `src-tauri/src/nlp/chat.rs`
**Current:** Has model field in ConversationSettings
```rust
pub struct ConversationSettings {
    pub model: String,  // Already exists, good!
    pub temperature: f32,
    pub max_tokens: u32,
    pub system_prompt: Option<String>,
}
```
**Impact:** LOW - Minor enhancement needed
**Changes:**
- Add `provider: String` field to `ConversationSettings`
- Update `Default` implementation
- No breaking changes to existing code

---

#### 9. `src-tauri/src/storage/chat_store.rs`
**Current:** Stores settings as JSON
**Impact:** LOW - Add migration
**Changes:**
- Add migration for `provider` field in settings JSON
- Handle old records without provider field (default to "anthropic")

---

### Summary of Impact Levels

| File | Impact | Risk | Effort |
|------|--------|------|--------|
| `ai/mod.rs` | HIGH | Low | 1-2h |
| `ai/claude.rs` | HIGH | Medium | 4-6h |
| `commands/nlp.rs` | HIGH | Medium | 4-6h |
| `nlp/classifier.rs` | MEDIUM | Low | 2-3h |
| `nlp/context_manager.rs` | MEDIUM | Low | 2-3h |
| `nlp/multi_detector.rs` | MEDIUM | Low | 1-2h |
| `lib.rs` | HIGH | Medium | 2-3h |
| `nlp/chat.rs` | LOW | Low | 0.5h |
| `storage/chat_store.rs` | LOW | Low | 1h |

**Total Estimated Effort for Modifications:** 18-27 hours

---

### Backward Compatibility

**Breaking Changes:**
- `ClaudeClient::new()` signature may change
- `NlpState::new()` signature will change
- Error types will change from `ClaudeError` to `ProviderError`

**Migration Strategy:**
1. Create new `LlmProvider` trait without modifying existing code
2. Implement trait for `ClaudeClient` (adapter pattern)
3. Update consumers one at a time
4. Add config system in parallel
5. Switch over initialization in `lib.rs` last

---

## Analysis & Current State

**Date:** 2026-03-01

### Existing Implementation

#### Backend (Rust/Tauri)
| Component | File | Status | Notes |
|-----------|------|--------|-------|
| Claude Client | `src-tauri/src/ai/claude.rs` | ✅ Exists | Single provider only, hardcoded model |
| AI Module | `src-tauri/src/ai/mod.rs` | ✅ Exists | Only exports Claude |
| NLP Commands | `src-tauri/src/commands/nlp.rs` | ⚠️ Partial | Uses hardcoded `"dev-api-key"` |
| Chat Store | `src-tauri/src/storage/chat_store.rs` | ✅ Exists | SQLite persistence ready |
| Chat Types | `src-tauri/src/nlp/chat.rs` | ✅ Exists | `ConversationSettings` has model field |

#### Frontend (TypeScript/Svelte)
| Component | File | Status | Notes |
|-----------|------|--------|-------|
| NLP API | `src-ui/src/lib/api/nlp.ts` | ✅ Exists | No LLM config commands |
| NLP Types | `src-ui/src/lib/types/nlp.ts` | ✅ Exists | No provider/model types |

### Gap Analysis

| Requirement | Current State | Gap |
|-------------|---------------|-----|
| Multi-provider support | Only Claude | Need provider abstraction layer |
| API token storage | Hardcoded string | Need secure storage (keychain/encrypted) |
| Token management UI | None | Need settings UI component |
| Model selection | Hardcoded default | Need dynamic model discovery/selection |
| Provider configuration | None | Need config module |
| Rate limiting | None | Need request throttling |

### Open Questions

1. **Secure Storage Strategy**
   - Option A: Use OS keychain via `keyring-rs` crate (recommended for security)
   - Option B: Encrypted SQLite column with app-level encryption
   - **Recommendation:** Option A - leverages OS security, follows best practices

2. **Provider Priority**
   - P0: Anthropic Claude (already implemented)
   - P0: OpenAI (largest market share)
   - P1: Google Gemini
   - P1: Local models (Ollama integration)
   - **Decision needed:** Which 2nd provider for P0?

3. **Model Selection Scope**
   - Per-conversation (current `ConversationSettings.model` suggests this)
   - Global default with per-conversation override
   - **Recommendation:** Global default + per-conversation override

4. **Configuration Persistence**
   - Store in existing SQLite DB vs separate config file
   - **Recommendation:** SQLite for consistency with existing architecture

5. **Rate Limiting Strategy**
   - Per-provider limits tracking
   - User notification on limit approach
   - Queue requests or reject?

---

## Implementation Plan

### Phase 1: Provider Abstraction (Week 1-2)

**Goal:** Create a unified interface for multiple LLM providers

#### 1.1 Core Provider Trait
- [ ] Create `src-tauri/src/ai/provider.rs` with `LlmProvider` trait
- [ ] Define common types: `CompletionRequest`, `CompletionResponse`, `ModelInfo`
- [ ] Add provider error handling

#### 1.2 Refactor Claude Client
- [ ] Implement `LlmProvider` trait for `ClaudeClient`
- [ ] Extract model list functionality
- [ ] Add streaming support preparation

#### 1.3 Add OpenAI Provider (P0)
- [ ] Create `src-tauri/src/ai/openai.rs`
- [ ] Implement `LlmProvider` trait
- [ ] Handle OpenAI-specific API format

### Phase 2: Configuration System (Week 2-3)

**Goal:** Secure storage and management of API credentials

#### 2.1 Configuration Module
- [ ] Create `src-tauri/src/config/mod.rs`
- [ ] Create `src-tauri/src/config/providers.rs` for provider settings
- [ ] Create `src-tauri/src/config/settings.rs` for app-wide settings

#### 2.2 Secure Token Storage
- [ ] Add `keyring` crate dependency
- [ ] Create `src-tauri/src/config/secrets.rs` for secure credential storage
- [ ] Implement token validation API

#### 2.3 Configuration Persistence
- [ ] Create `settings` table in SQLite
- [ ] Create `provider_configs` table in SQLite
- [ ] Add migration for existing DBs

#### 2.4 Tauri Commands
- [ ] Add config commands to `src-tauri/src/commands/config.rs`:
  - `get_provider_config`
  - `set_provider_config`
  - `validate_api_key`
  - `list_available_models`
  - `get_default_model` / `set_default_model`

### Phase 3: Model Selection (Week 3-4)

**Goal:** Dynamic model discovery and selection UI

#### 3.1 Model Registry
- [ ] Create `src-tauri/src/ai/models.rs` for model management
- [ ] Implement model capability flags (streaming, vision, etc.)
- [ ] Add model pricing/cost metadata

#### 3.2 Frontend Types
- [ ] Add to `src-ui/src/lib/types/config.ts`:
  - `LlmProvider` type
  - `ProviderConfig` interface
  - `ModelInfo` interface
  - `AppSettings` interface

#### 3.3 Frontend API
- [ ] Create `src-ui/src/lib/api/config.ts`:
  - Provider config functions
  - Model listing functions
  - Settings management

#### 3.4 Settings UI Component
- [ ] Create `src-ui/src/lib/components/settings/` directory
- [ ] Create `ProviderSettings.svelte` component
- [ ] Create `ModelSelector.svelte` component
- [ ] Create `SettingsPage.svelte` (container)

---

## Frontend Impact Analysis

### Current UI Architecture

The application uses a **three-panel layout**:
1. **Left Sidebar** (`ConversationSidebar`) - Conversation list with search
2. **Main Content** (`ChatArea` / `ThreadPanel`) - Messages and input
3. **Right Panel** (`ThreadView`) - Thread navigation (conditional)

**Key Files:**
- `src-ui/src/routes/+page.svelte` - Main page with mock data
- `src-ui/src/lib/components/layout/ChatLayout.svelte` - Responsive layout
- `src-ui/src/lib/components/chat/sidebar/ConversationSidebar.svelte` - Sidebar
- `src-ui/src/lib/types/thread.ts` - `ConversationSettings` interface

### Existing Settings Structure

```typescript
// Already exists in thread.ts
interface ConversationSettings {
  model: string;        // ← Already has model field
  temperature: number;
  maxTokens: number;
  systemPrompt?: string;
}
```

**Gap:** No `provider` field, no global settings, no UI for configuration.

---

### User Flow Analysis

#### Flow 1: First-Time Setup (No API Key)
```
App Launch → Detect No API Key → Show Setup Modal
                                    ↓
                            Select Provider
                                    ↓
                            Enter API Key
                                    ↓
                            Validate → Success → Start Chat
                                     ↓
                                   Error → Retry
```

#### Flow 2: Changing Provider/Model
```
Settings Button → Settings Modal/Panel
                        ↓
                ┌───────┴───────┐
                ↓               ↓
         Provider Tab      Model Tab
                ↓               ↓
         Add/Edit Keys    Select Default
                ↓               ↓
         Validate         Per-Conversation
                                Override
```

#### Flow 3: Per-Conversation Model Override
```
Conversation Header → Model Dropdown → Select Model → Apply
                                                ↓
                                    Updates ConversationSettings
```

---

### UI Components Required

#### 1. Settings Entry Point

**Location Options:**
| Option | Pros | Cons |
|--------|------|------|
| Sidebar footer | Consistent with chat apps | Limited space |
| Header gear icon | Visible, accessible | Clutters header |
| Keyboard shortcut | Power user friendly | Hidden |
| **Recommendation:** Sidebar footer + keyboard shortcut (`Cmd+,`) |

**Component:** `SettingsButton.svelte`
- Add to `ConversationSidebar` footer
- Opens settings modal/panel

---

#### 2. Settings Modal/Panel

**Structure:**
```
┌─────────────────────────────────────────────────┐
│ Settings                                    [X] │
├─────────────────────────────────────────────────┤
│ [Providers] [Models] [General]                  │
├─────────────────────────────────────────────────┤
│                                                 │
│  PROVIDERS TAB                                  │
│  ┌─────────────────────────────────────────────┐│
│  │ Anthropic Claude                      [Edit]││
│  │ Status: ● Connected                         ││
│  │ Models: claude-3-opus, claude-3-sonnet      ││
│  └─────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────┐│
│  │ OpenAI                                [Add] ││
│  │ Status: ○ Not configured                    ││
│  └─────────────────────────────────────────────┘│
│                                                 │
│  [+ Add Provider]                               │
│                                                 │
├─────────────────────────────────────────────────┤
│                              [Cancel] [Save]    │
└─────────────────────────────────────────────────┘
```

**Components:**
- `SettingsModal.svelte` - Container with tabs
- `ProviderTab.svelte` - Provider list and management
- `ModelTab.svelte` - Model selection and defaults
- `GeneralTab.svelte` - App settings (future)

---

#### 3. API Key Input Modal

**Triggered when:** Adding new provider or editing existing

```
┌─────────────────────────────────────────────────┐
│ Configure Anthropic Claude                      │
├─────────────────────────────────────────────────┤
│                                                 │
│  API Key                                        │
│  ┌─────────────────────────────────────────────┐│
│  │ sk-ant-•••••••••••••••••••••••••      [👁] ││
│  └─────────────────────────────────────────────┘│
│                                                 │
│  ℹ️ Your API key is stored securely in your     │
│     system keychain and never sent to our       │
│     servers.                                    │
│                                                 │
│  [Validate Key]                                 │
│                                                 │
├─────────────────────────────────────────────────┤
│                    [Cancel] [Save]              │
└─────────────────────────────────────────────────┘
```

**Components:**
- `ApiKeyInput.svelte` - Secure input with validation
- `ProviderIcon.svelte` - Provider logo/indicator

---

#### 4. Model Selector (Inline)

**Location:** Conversation header or input area

**Option A: In Conversation Header**
```
┌─────────────────────────────────────────────────┐
│ JavaScript Algorithms    [claude-3-opus ▾]      │
└─────────────────────────────────────────────────┘
```

**Option B: In Input Area**
```
┌─────────────────────────────────────────────────┐
│                                                 │
│  [Message input...]                             │
│                                                 │
├─────────────────────────────────────────────────┤
│ [claude-3-opus ▾]               [Send]          │
└─────────────────────────────────────────────────┘
```

**Recommendation:** Option B - More discoverable, contextual

**Components:**
- `ModelSelector.svelte` - Dropdown with provider grouping
- `ModelBadge.svelte` - Display current model with provider color

---

#### 5. First-Run Setup Wizard

**Triggered when:** No API keys configured

```
┌─────────────────────────────────────────────────┐
│                                                 │
│            Welcome to Coder                     │
│                                                 │
│  Choose your AI provider to get started:        │
│                                                 │
│  ┌─────────────┐  ┌─────────────┐              │
│  │  Anthropic  │  │   OpenAI    │              │
│  │   Claude    │  │   GPT-4     │              │
│  │   [●]       │  │   [ ]       │              │
│  └─────────────┘  └─────────────┘              │
│                                                 │
│  API Key: [________________________]            │
│                                                 │
│  [Skip for now]              [Continue]         │
│                                                 │
└─────────────────────────────────────────────────┘
```

**Components:**
- `SetupWizard.svelte` - Full-screen modal
- `ProviderCard.svelte` - Selectable provider option

---

### Layout Impact

#### ConversationSidebar Changes

**Current Structure:**
```
┌──────────────────┐
│ Header (title +) │
│ SearchBar        │
│ ─────────────────│
│ ConversationList │
│                  │
│                  │
└──────────────────┘
```

**New Structure:**
```
┌──────────────────┐
│ Header (title +) │
│ SearchBar        │
│ ─────────────────│
│ ConversationList │
│                  │
├──────────────────┤
│ Settings Button  │ ← NEW
│ Model Indicator  │ ← NEW (current default)
└──────────────────┘
```

**Files to Modify:**
- `ConversationSidebar.svelte` - Add footer section
- `ConversationSidebar.module.css` - Footer styles

---

### Files to Modify (Frontend)

| File | Impact | Changes |
|------|--------|---------|
| `types/thread.ts` | LOW | Add `provider` to `ConversationSettings` |
| `components/chat/sidebar/ConversationSidebar.svelte` | MEDIUM | Add settings footer |
| `components/chat/input/ChatInput.svelte` | MEDIUM | Add model selector |
| `routes/+page.svelte` | LOW | Add settings modal trigger |
| `routes/+layout.svelte` | LOW | Add setup wizard check |

---

### New Files to Create (Frontend)

```
src-ui/src/lib/
├── api/
│   └── config.ts                    # Config API client
├── types/
│   └── config.ts                    # Config types
├── stores/
│   └── settings.ts                  # Settings store (reactive)
└── components/
    └── settings/
        ├── SettingsModal.module.css
        ├── SettingsModal.svelte
        ├── ProviderTab.module.css
        ├── ProviderTab.svelte
        ├── ModelTab.module.css
        ├── ModelTab.svelte
        ├── ApiKeyInput.module.css
        ├── ApiKeyInput.svelte
        ├── ModelSelector.module.css
        ├── ModelSelector.svelte
        ├── ModelBadge.module.css
        ├── ModelBadge.svelte
        ├── SetupWizard.module.css
        └── SetupWizard.svelte
```

---

### Design System Considerations

**Use existing tokens:**
- Colors: `--color-primary-*` for provider accents
- Status: `--color-success` for connected, `--color-error` for failed
- Spacing: `--space-*` for consistent padding
- Radius: `--radius-lg` for cards, `--radius-md` for inputs
- Shadows: `--shadow-lg` for modals

**Provider Colors (suggested):**
```css
--color-provider-anthropic: #d97706;  /* Orange */
--color-provider-openai: #10b981;     /* Green */
--color-provider-google: #4285f4;     /* Blue */
```

---

### Accessibility Requirements

1. **Keyboard Navigation:**
   - Tab through settings fields
   - Enter to submit
   - Escape to close modals

2. **Screen Reader:**
   - Announce provider connection status
   - Announce validation results
   - Label all form fields

3. **Focus Management:**
   - Trap focus in modals
   - Return focus on close
   - Focus first field on open

---

### State Management

**New Store:** `settingsStore.ts`

```typescript
interface SettingsState {
  // Provider configs
  providers: ProviderConfig[];
  activeProvider: string | null;

  // Model settings
  defaultModel: string;
  models: ModelInfo[];

  // UI state
  setupComplete: boolean;
  settingsOpen: boolean;
}
```

**Reactive Behavior:**
- Provider change → Re-fetch models
- Model change → Update conversations using default
- API key validation → Update connection status

---

### Phase 4: Integration & Testing (Week 4-5)

**Goal:** Connect all components and validate

#### 4.1 Integration
- [ ] Update `NlpState` to use provider registry
- [ ] Update `ConversationSettings` to support model selection
- [ ] Wire up settings UI to backend

#### 4.2 Testing
- [ ] Unit tests for provider trait implementations
- [ ] Unit tests for config storage
- [ ] Integration tests for token validation
- [ ] E2E tests for model switching

#### 4.3 Documentation
- [ ] Document provider configuration process
- [ ] Document model selection behavior
- [ ] Add troubleshooting guide for common API errors

---

## Files Summary

### Backend Files to Create

```
src-tauri/src/
├── ai/
│   ├── provider.rs      # LlmProvider trait
│   ├── openai.rs        # OpenAI implementation
│   └── models.rs        # Model registry
├── config/
│   ├── mod.rs           # Config module
│   ├── providers.rs     # Provider settings
│   ├── settings.rs      # App settings
│   └── secrets.rs       # Secure token storage
└── commands/
    └── config.rs        # Config Tauri commands
```

### Backend Files to Modify

| File | Impact |
|------|--------|
| `ai/mod.rs` | HIGH - Add provider exports |
| `ai/claude.rs` | HIGH - Implement LlmProvider trait |
| `commands/nlp.rs` | HIGH - Use ProviderRegistry |
| `nlp/classifier.rs` | MEDIUM - Use dyn LlmProvider |
| `nlp/context_manager.rs` | MEDIUM - Use dyn LlmProvider |
| `nlp/multi_detector.rs` | MEDIUM - Use dyn LlmProvider |
| `nlp/chat.rs` | LOW - Add provider field |
| `lib.rs` | HIGH - New initialization flow |
| `storage/chat_store.rs` | LOW - Migration |

### Frontend Files to Create

```
src-ui/src/lib/
├── api/
│   └── config.ts                    # Config API client
├── types/
│   └── config.ts                    # Config types
├── stores/
│   └── settings.ts                  # Settings store
└── components/
    └── settings/
        ├── SettingsModal.module.css
        ├── SettingsModal.svelte
        ├── ProviderTab.module.css
        ├── ProviderTab.svelte
        ├── ModelTab.module.css
        ├── ModelTab.svelte
        ├── ApiKeyInput.module.css
        ├── ApiKeyInput.svelte
        ├── ModelSelector.module.css
        ├── ModelSelector.svelte
        ├── ModelBadge.module.css
        ├── ModelBadge.svelte
        ├── SetupWizard.module.css
        └── SetupWizard.svelte
```

### Frontend Files to Modify

| File | Impact |
|------|--------|
| `types/thread.ts` | LOW - Add provider to ConversationSettings |
| `components/chat/sidebar/ConversationSidebar.svelte` | MEDIUM - Add settings footer |
| `components/chat/input/ChatInput.svelte` | MEDIUM - Add model selector |
| `routes/+page.svelte` | LOW - Add settings modal |
| `routes/+layout.svelte` | LOW - Add setup wizard check |

---

## Dependencies to Add

```toml
# Cargo.toml
keyring = "2.0"  # OS keychain access
```

---

## Effort Summary

| Phase | Backend | Frontend | Total |
|-------|---------|----------|-------|
| Phase 1: Provider Abstraction | 12-16h | - | 12-16h |
| Phase 2: Configuration System | 10-14h | - | 10-14h |
| Phase 3: Model Selection | 4-6h | 16-24h | 20-30h |
| Phase 4: Integration & Testing | 8-12h | 4-6h | 12-18h |
| **Total** | **34-48h** | **20-30h** | **54-78h** |

---

## Success Metrics

- [ ] Can configure 2+ LLM providers with secure token storage
- [ ] Can switch models per conversation
- [ ] API keys never logged or exposed in plaintext
- [ ] Token validation provides immediate feedback
- [ ] Model list updates dynamically from provider APIs
- [ ] First-run setup wizard guides new users
- [ ] Settings accessible from sidebar + keyboard shortcut