# Implementation Plan: LLM Configuration Flow

> **Status:** READY TO IMPLEMENT
> **Generated:** 2026-03-01
> **Prototype Source:** prototypes/config-flow/

## ⚠️ MANDATORY: Prototype Reference

**Before implementing ANY component, the developer MUST reference the approved prototype.**

The prototype at `prototypes/config-flow/` is the **source of truth** for:
- Visual design and styling
- Component structure and hierarchy
- Interaction patterns and behaviors
- Animation and transition effects
- State management logic

### Prototype Files Reference

| Prototype File | Purpose | Implementation Reference |
|----------------|---------|-------------------------|
| `index.html` | Component structure, Alpine.js directives | Convert HTML structure to Svelte, map Alpine.js to Svelte patterns |
| `css/components.css` | All component styles | Convert to CSS modules, use design tokens |
| `css/design-system.css` | Design tokens | Already exists in project at `src-ui/src/lib/styles/design-system.css` |
| `css/animations.css` | Animation keyframes | Add to global styles or component CSS |
| `js/state.js` | State management, methods | Convert Alpine.js state to Svelte 5 `$state` runes |
| `README.md` | Feature specification, user journeys | Reference for behavior and requirements |

### Component-to-Prototype Mapping

| Component | Prototype Location (index.html) | CSS Source (components.css) |
|-----------|--------------------------------|----------------------------|
| Modal base | `.modal-overlay`, `.modal` lines 333-432 | Modal structure, overlay, header, body, footer |
| Button variants | `.btn`, `.btn-primary`, `.btn-secondary` lines 1064-1124 | All button states and variants |
| Input | `.form-input`, `.form-group`, `.form-label` lines 725-831 | Input styling, labels, helpers |
| ProvidersModal | lines 190-257 | `.provider-grid`, `.provider-card` lines 546-656 |
| ConfigModal | lines 262-311 | `.config-header`, `.validation-msg` lines 695-866 |
| CustomProviderModal | lines 316-440 | `.compatibility-options` lines 1676-1765 |
| SettingsModal | lines 445-568 | `.settings-section`, `.operation-group` lines 1218-1496 |
| ProviderCard | lines 210-229 | `.provider-card` lines 553-656 |
| ModelCard | lines 524-537 | `.settings-model-card` lines 1402-1496 |
| InfoBanner | lines 153-156 | `.info-banner` lines 483-511 |

### Pre-Implementation Checklist

Before starting implementation, verify:

- [ ] Open `prototypes/config-flow/index.html` in browser to see the working prototype
- [ ] Read `prototypes/config-flow/README.md` for full feature specification
- [ ] Review `prototypes/config-flow/css/components.css` for exact styling
- [ ] Review `prototypes/config-flow/js/state.js` for state management patterns
- [ ] Keep prototype files open for reference during implementation

---

## Summary

Implement the LLM provider and model configuration flow for the Coder AI desktop application. This includes modals for configuring AI providers (Anthropic, OpenAI, Google, Custom), managing API keys, and selecting default models for different operation types. The feature requires both frontend Svelte components and backend Tauri commands for secure API key storage and provider persistence.

---

## Scope

### In-Scope Components
| Component | Description |
|-----------|-------------|
| Providers Modal | Grid of provider cards with configuration status |
| Config Modal | API key input and validation for a provider |
| Custom Provider Modal | Form to add custom API-compatible providers |
| Settings Modal | Manage providers and assign models per operation |
| Provider Card | Clickable card showing provider info and status |
| Model Card | Selectable card for model selection |
| Info Banner | Informational banner component |

### Contextual (Not in Scope)
- Welcome Modal
- Flow Indicator / Progress Bar
- Flow Navigation
- App Shell (Sidebar, Chat Interface)

---

## Data Model

### SQLite Tables

```sql
-- Providers (user-configured AI providers)
CREATE TABLE providers (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    icon          TEXT,
    is_custom     BOOLEAN DEFAULT FALSE,
    api_type      TEXT CHECK(api_type IN ('openai', 'anthropic')),
    base_url      TEXT,
    is_configured BOOLEAN DEFAULT FALSE,
    created_at    TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at    TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Models (fetched from provider APIs, cached in DB)
CREATE TABLE models (
    id                  TEXT PRIMARY KEY,
    provider_id         TEXT NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
    name                TEXT NOT NULL,
    supports_analysis   BOOLEAN DEFAULT TRUE,
    supports_code       BOOLEAN DEFAULT TRUE,
    supports_vision     BOOLEAN DEFAULT FALSE,
    supports_audio      BOOLEAN DEFAULT FALSE,
    supports_tips       BOOLEAN DEFAULT TRUE,
    supports_review     BOOLEAN DEFAULT TRUE,
    last_fetched        TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at          TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Operation defaults (user's selected model per operation)
CREATE TABLE operation_defaults (
    operation   TEXT PRIMARY KEY CHECK(operation IN ('analysis', 'code', 'vision', 'audio', 'tips', 'review')),
    model_id    TEXT NOT NULL REFERENCES models(id) ON DELETE SET NULL
);
```

### OS Keychain Storage
- API keys stored per provider: `{provider_id}_api_key`

---

## Frontend Implementation

### Shared Components (New)

| Component | Path | Description | Priority |
|-----------|------|-------------|----------|
| Modal | `src-ui/src/lib/components/shared/Modal.svelte` | Reusable modal with overlay, header, body, footer | P1 |
| Button | `src-ui/src/lib/components/shared/Button.svelte` | Primary, secondary, ghost variants | P1 |
| Input | `src-ui/src/lib/components/shared/Input.svelte` | Text/password input with label and helper | P1 |
| IconButton | `src-ui/src/lib/components/shared/IconButton.svelte` | Button with icon only | P2 |

### Feature Components (New)

| Component | Path | Description | Priority |
|-----------|------|-------------|----------|
| ProvidersModal | `src-ui/src/lib/components/config/ProvidersModal.svelte` | Main provider selection modal | P1 |
| ConfigModal | `src-ui/src/lib/components/config/ConfigModal.svelte` | API key configuration for provider | P1 |
| CustomProviderModal | `src-ui/src/lib/components/config/CustomProviderModal.svelte` | Add custom provider form | P2 |
| SettingsModal | `src-ui/src/lib/components/config/SettingsModal.svelte` | Provider & model management | P1 |
| ProviderCard | `src-ui/src/lib/components/config/ProviderCard.svelte` | Provider display card | P1 |
| ModelCard | `src-ui/src/lib/components/config/ModelCard.svelte` | Model selection card | P1 |
| InfoBanner | `src-ui/src/lib/components/config/InfoBanner.svelte` | Informational banner | P2 |
| OperationGroup | `src-ui/src/lib/components/config/OperationGroup.svelte` | Operation type with model grid | P1 |

### State Management

| State Property | Type | Svelte Pattern | Location |
|----------------|------|----------------|----------|
| `providers` | `Provider[]` | `$state` | `configStore.ts` |
| `selectedModels` | `Record<OperationType, string>` | `$state` | `configStore.ts` |
| `currentModal` | `'providers' \| 'config' \| 'custom' \| 'settings' \| null` | `$state` | Component local |
| `validationState` | `{ status, message, icon }` | `$state` | Component local |
| `isLoading` | `boolean` | `$state` | Component local |

### Types

```typescript
// src-ui/src/lib/types/config.ts

export type ApiType = 'openai' | 'anthropic';
export type OperationType = 'analysis' | 'code' | 'vision' | 'audio' | 'tips' | 'review';

export interface Provider {
  id: string;
  name: string;
  icon: string;
  isCustom: boolean;
  apiType: ApiType;
  baseUrl?: string;
  isConfigured: boolean;
}

export interface Model {
  id: string;
  providerId: string;
  name: string;
  capabilities: {
    analysis: boolean;
    code: boolean;
    vision: boolean;
    audio: boolean;
    tips: boolean;
    review: boolean;
  };
}

export interface OperationConfig {
  id: OperationType;
  name: string;
  description: string;
  icon: string;
  color: string;
}

export const OPERATION_TYPES: OperationConfig[] = [
  { id: 'analysis', name: 'Analysis', description: 'Simple models for quick questions', icon: 'search', color: 'var(--color-info)' },
  { id: 'code', name: 'Code Generation', description: 'Powerful models for generating code', icon: 'code', color: 'var(--color-warning)' },
  { id: 'vision', name: 'Images & Vision', description: 'Models with vision capability', icon: 'eye', color: 'var(--color-primary-500)' },
  { id: 'audio', name: 'Audio & Transcription', description: 'Audio recording and transcription', icon: 'mic', color: 'var(--color-success)' },
  { id: 'tips', name: 'Tips & Refinements', description: 'Quick suggestions and improvements', icon: 'lightbulb', color: 'var(--color-warning)' },
  { id: 'review', name: 'Code Review', description: 'Quality & best practices', icon: 'check-circle', color: 'var(--color-primary-500)' },
];
```

### CSS Updates

| File | Changes |
|------|---------|
| `Modal.module.css` | New file - modal overlay, container, header, body, footer styles |
| `Button.module.css` | New file - primary, secondary, ghost button variants |
| `Input.module.css` | New file - form input with label, helper, validation states |
| `ProviderCard.module.css` | New file - provider card styles from prototype |
| `ModelCard.module.css` | New file - model card styles from prototype |
| `InfoBanner.module.css` | New file - banner component styles |
| `animations.css` | Add to global styles - fadeIn, slideUp, spin keyframes |

### Animation Approach (Hybrid)
- **CSS animations**: fadeIn, slideUp, spin (simple, reusable)
- **Svelte transitions**: Modal enter/leave transitions (fly, fade)

### Implementation Steps

#### Phase 1: Shared Components
- [ ] Create `Modal.svelte` with overlay, close button, slots
- [ ] Create `Button.svelte` with variants (primary, secondary, ghost, danger)
- [ ] Create `Input.svelte` with label, helper text, validation states
- [ ] Add CSS animation keyframes to global styles
- [ ] Write unit tests for shared components

#### Phase 2: Feature Components
- [ ] Create `InfoBanner.svelte` component
- [ ] Create `ProviderCard.svelte` with hover/selected states
- [ ] Create `ModelCard.svelte` with selection indicator
- [ ] Create `OperationGroup.svelte` for model selection per operation
- [ ] Create `ProvidersModal.svelte` with provider grid
- [ ] Create `ConfigModal.svelte` for API key input
- [ ] Create `CustomProviderModal.svelte` form
- [ ] Create `SettingsModal.svelte` combining all sections

#### Phase 3: Store & Integration
- [ ] Create `configStore.ts` with providers and selected models state
- [ ] Create Tauri API wrapper functions in `src-ui/src/lib/api/config.ts`
- [ ] Integrate modals with application state
- [ ] Add keyboard navigation (ESC to close, Enter to submit)

#### Phase 4: Testing & Polish
- [ ] Add accessibility attributes (ARIA labels, roles)
- [ ] Test keyboard navigation
- [ ] Test screen reader announcements
- [ ] Verify color contrast ratios
- [ ] Write component tests

---

## Backend Implementation (Tauri)

> **Status:** REQUIRED - See separate plan: `backend-implementation-plan.md`

### Tauri Commands

| Command | File | Description |
|---------|------|-------------|
| `get_providers` | `commands/providers.rs` | Fetch all configured providers |
| `save_provider` | `commands/providers.rs` | Create or update a provider |
| `remove_provider` | `commands/providers.rs` | Delete a custom provider |
| `validate_api_key` | `commands/providers.rs` | Validate API key with provider |
| `test_connection` | `commands/providers.rs` | Test custom provider connection |
| `fetch_models` | `commands/providers.rs` | Fetch available models from provider API |
| `get_operation_defaults` | `commands/config.rs` | Get selected model per operation |
| `set_operation_default` | `commands/config.rs` | Set model for an operation |
| `store_api_key` | `commands/keychain.rs` | Store API key in OS keychain |
| `retrieve_api_key` | `commands/keychain.rs` | Retrieve API key from keychain |
| `delete_api_key` | `commands/keychain.rs` | Delete API key from keychain |

### Storage

| Store | File | Description |
|-------|------|-------------|
| `ProviderStore` | `storage/provider_store.rs` | SQLite operations for providers |
| `ModelStore` | `storage/model_store.rs` | SQLite operations for models |
| `ConfigStore` | `storage/config_store.rs` | SQLite operations for operation defaults |

### Dependencies (Rust)

```toml
[dependencies]
keyring = "2"          # OS keychain access
reqwest = { version = "0.11", features = ["json"] }  # HTTP client for API validation
serde_json = "1"       # JSON parsing for API responses
```

---

## File Changes Summary

### New Files

**Frontend:**
```
src-ui/src/lib/components/
├── shared/
│   ├── Modal.svelte
│   ├── Modal.module.css
│   ├── Button.svelte
│   ├── Button.module.css
│   ├── Input.svelte
│   ├── Input.module.css
│   └── IconButton.svelte
└── config/
    ├── ProvidersModal.svelte
    ├── ProvidersModal.module.css
    ├── ConfigModal.svelte
    ├── ConfigModal.module.css
    ├── CustomProviderModal.svelte
    ├── CustomProviderModal.module.css
    ├── SettingsModal.svelte
    ├── SettingsModal.module.css
    ├── ProviderCard.svelte
    ├── ProviderCard.module.css
    ├── ModelCard.svelte
    ├── ModelCard.module.css
    ├── InfoBanner.svelte
    ├── InfoBanner.module.css
    ├── OperationGroup.svelte
    └── OperationGroup.module.css

src-ui/src/lib/
├── stores/
│   └── configStore.ts
├── types/
│   └── config.ts
└── api/
    └── config.ts
```

**Backend:**
```
src-tauri/src/
├── commands/
│   ├── providers.rs
│   ├── config.rs
│   └── keychain.rs
└── storage/
    ├── provider_store.rs
    ├── model_store.rs
    └── config_store.rs
```

### Modified Files

| File | Changes |
|------|---------|
| `src-tauri/src/lib.rs` | Register new commands |
| `src-tauri/src/commands/mod.rs` | Export new command modules |
| `src-tauri/src/storage/mod.rs` | Export new store modules |
| `src-tauri/Cargo.toml` | Add keyring, reqwest dependencies |
| `src-ui/src/lib/styles/global.css` | Add animation keyframes |

---

## Risks & Considerations

1. **API Key Security**: Ensure keys are never logged or exposed in error messages
2. **API Rate Limiting**: Provider validation may hit rate limits - implement caching
3. **Model Capability Detection**: Some providers may not expose model capabilities - may need manual config
4. **Offline Mode**: Consider graceful degradation when API validation is not possible
5. **Custom Provider Compatibility**: Different providers may have slight API variations

---

## Acceptance Criteria

### Functional Requirements
- [ ] User can configure multiple AI providers (Anthropic, OpenAI, Google)
- [ ] User can add custom providers with OpenAI or Anthropic compatibility
- [ ] API keys are stored securely in OS keychain
- [ ] API key validation works for all provider types
- [ ] User can select default models for each operation type
- [ ] Provider configuration persists across app restarts
- [ ] All modals are keyboard accessible (Tab, Escape, Enter)
- [ ] Screen readers announce state changes correctly

### Prototype Compliance (MANDATORY)
- [ ] Visual design matches prototype exactly (spacing, colors, typography)
- [ ] Component structure follows prototype HTML structure
- [ ] Hover states match prototype (borders, transforms, shadows)
- [ ] Selected/active states match prototype (gradients, indicators)
- [ ] Animations match prototype timing and easing
- [ ] Modal flow matches prototype user journeys
- [ ] Validation states match prototype (info, success, error)

### Code Quality
- [ ] CSS uses design tokens (no hardcoded colors/spacing)
- [ ] Components follow project file size limits (<500 lines)
- [ ] No inline styles or scripts
- [ ] Alpine.js patterns correctly converted to Svelte 5 runes

---

## Prototype Verification Checklist

After implementation, compare each component against the prototype:

### Visual Verification
Open the prototype at `prototypes/config-flow/index.html` and the implemented component side-by-side:

| Component | Prototype State | Implementation Match |
|-----------|-----------------|---------------------|
| ProviderCard | Default, Hover, Configured | [ ] Verified |
| ModelCard | Default, Hover, Selected | [ ] Verified |
| Button (Primary) | Default, Hover, Disabled | [ ] Verified |
| Button (Secondary) | Default, Hover | [ ] Verified |
| Input | Default, Focus, Error | [ ] Verified |
| InfoBanner | Default | [ ] Verified |
| Modal | Overlay, Container | [ ] Verified |
| ValidationMessage | Info, Success, Error | [ ] Verified |

### Interaction Verification
- [ ] Provider card hover effect matches prototype (lift, border color)
- [ ] Model card selection matches prototype (gradient, check icon)
- [ ] Button hover states match prototype
- [ ] Input focus state matches prototype (glow shadow)
- [ ] Modal slide-up animation matches prototype
- [ ] Validation state transitions match prototype

### State Verification
Compare state behavior from `prototypes/config-flow/js/state.js`:
- [ ] `configuredCount` computed correctly
- [ ] `displayModelProvider` returns correct provider class
- [ ] `displayModelName` returns correct model name
- [ ] `getAvailableModels` filters by operation capability
- [ ] `validateApiKey` shows correct validation states
- [ ] `saveProvider` updates configured status
