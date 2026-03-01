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

## Files to Create

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

src-ui/src/lib/
├── api/
│   └── config.ts        # Config API client
├── types/
│   └── config.ts        # Config types
└── components/
    └── settings/
        ├── ProviderSettings.svelte
        ├── ModelSelector.svelte
        └── SettingsPage.svelte
```

## Dependencies to Add

```toml
# Cargo.toml
keyring = "2.0"  # OS keychain access
```

---

## Success Metrics

- [ ] Can configure 2+ LLM providers with secure token storage
- [ ] Can switch models per conversation
- [ ] API keys never logged or exposed in plaintext
- [ ] Token validation provides immediate feedback
- [ ] Model list updates dynamically from provider APIs