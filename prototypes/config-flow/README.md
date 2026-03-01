# LLM Configuration Flow - Feature Design Specification

## Overview

This specification defines the LLM provider and model configuration flow for the Coder AI desktop application. The configuration flow enables users to set up multiple AI providers, configure custom providers, and assign default models for different operation types.

> **Note**: This prototype focuses on the **configuration flow modals** (Welcome, Providers, Config, Custom Provider, and Settings). The app shell (sidebar, chat area, input field) shown in the prototype is purely contextual backdrop and not part of the feature scope.

### Business Context

Users need flexibility in choosing AI providers based on their requirements, budget, and existing infrastructure. The configuration flow provides a streamlined onboarding experience while maintaining power-user flexibility for advanced configurations.

## User Stories

### Primary Users

**As a Developer**, I want to configure multiple AI providers so that I can:
- Switch between providers based on task requirements
- Use different providers for different operations (e.g., fast models for analysis, powerful models for code generation)
- Maintain redundancy in case one provider is unavailable

**As a Power User**, I want to add custom providers so that I can:
- Use self-hosted or local models (e.g., Ollama, vLLM)
- Connect to internal company API endpoints
- Control costs by using alternative providers

**As a New User**, I want a simple onboarding flow so that I can:
- Get started quickly with minimal configuration
- Understand what I need to configure before beginning
- Skip setup and configure later if needed

## Functional Requirements

### FR-1: Multi-Provider Configuration
The system MUST allow users to configure multiple AI providers simultaneously. Each provider requires:
- Provider selection (Anthropic, OpenAI, Google, or Custom)
- API key input with secure storage
- Visual indication of configuration status

### FR-2: Custom Provider Support
The system MUST support adding custom providers with:
- Custom provider name
- API compatibility type selection (OpenAI or Anthropic compatible)
- Optional custom base URL
- Configurable model list

### FR-3: Operation-Based Model Selection
The system MUST allow users to assign different default models for different operation types:
- **Analysis**: Quick questions and simple tasks
- **Code Generation**: Complex code writing and generation
- **Vision**: Image and screenshot analysis
- **Audio & Transcription**: Audio recording and transcription models
- **Tips & Refinements**: Quick suggestions and improvements
- **Code Review**: Quality analysis and best practices

### FR-4: Navigation and Flow Control
The system MUST provide:
- Multi-step flow with clear progress indication
- Ability to skip setup and configure later
- Quick navigation between flow steps
- Access to settings from main application

### FR-5: API Key Management
The system MUST:
- Store API keys securely in OS keychain
- Provide API key validation before saving
- Show clear feedback for validation states
- Allow editing of existing API keys

### FR-6: Provider Management
The system MUST allow users to:
- Add new providers (built-in or custom)
- Edit existing provider configurations
- Remove custom providers
- View list of all configured providers

## UI Components

### Component Hierarchy

> **Scope Note**: Only the **Configuration Flow Modals** are in scope for this feature. The App Shell components are shown as contextual backdrop and are documented here for reference only.

```
[App Shell - Contextual Backdrop - Not in Scope]
├── Sidebar
│   ├── Sidebar Header (App Title)
│   ├── Sidebar Content (Conversations List)
│   └── Sidebar Footer (Current Model + Settings)
└── Main Content Area
    └── Chat Interface

[Configuration Flow Modals - IN SCOPE]
├── Flow Indicator (Progress Bar)
├── Flow Navigation (Quick Jump Buttons)
│
├── Welcome Modal
│   ├── Logo
│   ├── Title and Subtitle
│   ├── Info Banner
│   ├── Feature List
│   └── Footer Actions
│
├── Providers Modal
│   ├── Header (Title + Close)
│   ├── Configured Summary
│   ├── Provider Grid
│   │   ├── Provider Cards (Built-in)
│   │   └── Add Provider Card
│   └── Footer Actions
│
├── Config Modal (Individual Provider)
│   ├── Header (Provider Name + Close)
│   ├── Config Header (Icon + Name + Models)
│   ├── API Key Input
│   ├── Validation Message
│   ├── Security Info Banner
│   └── Footer Actions
│
├── Custom Provider Modal
│   ├── Header (Title + Close)
│   ├── Provider Name Input
│   ├── API Compatibility Selection
│   ├── Base URL Input (Optional)
│   ├── API Key Input
│   ├── Test Connection Button
│   ├── Fetch Models Button
│   ├── Models List Input
│   └── Footer Actions
│
└── Settings Modal
    ├── Header (Title + Close)
    ├── Providers Section
    │   ├── Section Header (Users Icon + Title + Add Button)
    │   └── Providers List
    │       └── Provider List Items
    │           ├── Provider Info
    │           └── Provider Actions (Edit + Remove)
    └── Models Section
        ├── Section Header (Box Icon + Title)
        └── Operation Groups
            ├── Operation Header (Icon + Name + Selected Model)
            └── Model Cards Grid
```

### Component Responsibilities

> **Note**: Components marked with ~~strikethrough~~ are contextual backdrop and not in scope.

| Component | Responsibility | Scope |
|-----------|---------------|-------|
| ~~Flow Indicator~~ | Shows current step in configuration process (1-4) with visual active/completed states | Contextual |
| ~~Flow Navigation~~ | Provides quick access to any step in the flow via floating button group | Contextual |
| ~~Welcome Modal~~ | Introduces the setup process, lists key features, and provides entry to configuration | Contextual |
| **Providers Modal** | Displays available providers as cards, shows configured status, allows adding custom providers | ✅ In Scope |
| **Config Modal** | Collects API key for a specific provider, validates input, shows security information | ✅ In Scope |
| **Custom Provider Modal** | Collects custom provider details, API key, tests connection, fetches available models | ✅ In Scope |
| **Settings Modal** | Central hub for managing all providers and configuring operation-based model assignments | ✅ In Scope |
| **Provider Card** | Clickable card showing provider name, available models, and configuration status | ✅ In Scope |
| **Model Card** | Selectable card showing model name, provider, and selection state | ✅ In Scope |
| **Info Banner** | Highlights important information with icon and styled background | ✅ In Scope |

## Interaction Flows

### Flow 1: Initial Setup (New User)

**Entry Point**: Application launch with no configured providers

**Steps**:
1. User launches Coder application
2. Welcome modal appears automatically
3. User reviews feature overview
4. User clicks "Get Started" or "Skip for now"
5. If "Get Started": Navigate to Providers modal
6. If "Skip for now": Navigate to main app (limited functionality)

### Flow 2: Provider Configuration

**Entry Point**: Welcome modal → "Get Started" OR Settings → "Add Provider"

**Steps**:
1. Providers modal displays available providers
2. User clicks on a provider card
3. Config modal opens for selected provider
4. User enters API key
5. User clicks "Validate" (optional)
6. System validates API key
7. User clicks "Save"
8. Config modal closes, provider card shows "Configured" status
9. User can configure additional providers or proceed

### Flow 3: Custom Provider Addition

**Entry Point**: Providers modal → "Add Provider" button OR Settings → "Add Provider"

**Steps**:
1. Custom Provider modal opens
2. User enters provider name
3. User selects API compatibility type (OpenAI or Anthropic)
4. User optionally enters custom base URL
5. User enters available models (comma-separated)
6. User clicks "Add Provider"
7. Provider is added to the providers list
8. User can configure the new provider's API key

### Flow 4: Model Assignment

**Entry Point**: Settings modal → Models Section

**Steps**:
1. Settings modal displays operation groups
2. For each operation type:
   - User views available models from configured providers
   - User clicks on a model card to select it
   - Selected model shows check indicator
   - Operation header shows selected model name
3. User clicks "Complete Setup" or "Save Changes"
4. Model assignments are saved

### Flow 5: Settings Access

**Entry Point**: Main app → Sidebar → "Settings" button

**Steps**:
1. Settings modal opens
2. User can manage providers (add, edit, remove)
3. User can configure model assignments
4. User clicks "Cancel" to discard changes or "Save Changes" to persist

## User Journeys and Logic

### Journey 1: Quick Start with Single Provider

**User Goal**: Get started quickly with minimal configuration

**Conditional Logic**:
- IF application launches with no configured providers, THEN display Welcome modal
- IF user clicks "Get Started", THEN navigate to Providers modal
- IF user clicks on a provider card, THEN open Config modal for that provider
- IF user enters valid API key and clicks "Save", THEN mark provider as configured and return to Providers modal
- IF at least one provider is configured, THEN enable "Continue to Settings" button
- IF user clicks "Continue to Settings", THEN navigate to Settings modal
- IF user selects at least one model, THEN enable "Complete Setup" button
- IF user clicks "Complete Setup", THEN navigate to main app with full functionality

**Flow Diagram**:
```
    [Launch App]
         │
         ▼
   [No Providers?]
         │
    ┌────┴────┐
    │         │
   [Yes]     [No]
    │         │
    ▼         ▼
[Welcome]──[Main App]
[Modal]     (Limited)
    │
    ▼
[Get Started?]
    │
┌───┴───┐
│       │
[Yes]  [Skip]
│       │
▼       ▼
[Providers]──[Main App]
[Modal]      (Limited)
    │
    ▼
[Select Provider]
    │
    ▼
[Enter API Key]
    │
    ▼
[Save → Configured]
    │
    ▼
[Continue to Settings]
    │
    ▼
[Select Models]
    │
    ▼
[Complete Setup]
    │
    ▼
   [Main App]
  (Full Functionality)
```

### Journey 2: Advanced Configuration with Custom Provider

**User Goal**: Add a custom self-hosted provider

**Conditional Logic**:
- IF user clicks "Add Provider" button, THEN open Custom Provider modal
- IF user enters provider name, THEN enable form validation
- IF user selects API compatibility type, THEN show appropriate endpoint description
- IF user enters custom base URL, THEN override default endpoint for that provider
- IF user enters models list, THEN parse comma-separated values as available models
- IF form is valid and user clicks "Add Provider", THEN create provider and add to list
- IF new provider is added, THEN display in providers list with "Not Configured" status
- IF user clicks on new provider, THEN open Config modal for API key entry
- IF API key is saved, THEN mark provider as configured

**Flow Diagram**:
```
      [Settings]
           │
           ▼
    [Click Add Provider]
           │
           ▼
    [Custom Provider Modal]
           │
           ▼
    [Enter Provider Name]
           │
           ▼
    [Select API Type]
       │     │
       ▼     ▼
  [OpenAI] [Anthropic]
       │     │
       └──┬──┘
          │
          ▼
    [Optional: Base URL]
          │
          ▼
    [Enter Models List]
          │
          ▼
    [Click Add Provider]
          │
          ▼
    [Provider Added to List]
          │
          ▼
    [Configure API Key]
          │
          ▼
    [Provider Ready to Use]
```

### Journey 3: Model Selection by Operation

**User Goal**: Configure optimal models for different task types

**Conditional Logic**:
- IF user opens Settings modal, THEN display all operation groups
- IF provider is configured, THEN show its available models in each operation
- IF operation has models available, THEN display model cards in grid
- IF no models available for operation, THEN show empty state message
- IF user clicks on a model card, THEN select that model for the operation
- IF model is selected, THEN show check indicator and update operation header
- IF user changes model selection, THEN replace previous selection
- IF all required operations have selections, THEN enable completion button

**Flow Diagram**:
```
        [Settings Modal]
              │
              ▼
        [Models Section]
              │
    ┌─────────┴─────────┐
    │                   │
[Analysis Group]  [Code Gen Group]
    │                   │
    ▼                   ▼
[Available Models] [Available Models]
    │                   │
    └─────────┬─────────┘
              │
              ▼
       [Click Model Card]
              │
              ▼
       [Model Selected]
              │
              ▼
    [Check Icon Shows]
              │
              ▼
    [Header Updates]
              │
              ▼
       [Repeat for Each
     Required Operation]
              │
              ▼
    [Complete Setup Enabled]
```

### Journey 4: Error Recovery

**User Goal**: Handle validation errors gracefully

**Conditional Logic**:
- IF user clicks "Validate" with empty API key, THEN show error message
- IF API key validation fails, THEN display error with retry option
- IF validation succeeds, THEN show success message and enable save
- IF user encounters errors, THEN provide clear next steps
- IF custom provider creation fails, THEN show specific error message

**Flow Diagram**:
```
       [Enter API Key]
             │
             ▼
      [Click Validate]
             │
      ┌──────┴──────┐
      │             │
   [Empty]     [Has Value]
      │             │
      ▼             ▼
  [Show Error] [API Validation]
      │             │
      │      ┌──────┴──────┐
      │      │             │
      │   [Valid]      [Invalid]
      │      │             │
      │      ▼             ▼
      │  [Show Success] [Show Error]
      │      │             │
      └──────┴─────────────┘
             │
             ▼
        [Fix and Retry]
```

## States and Variants

### Modal States

| Component | Default | Hover | Active/Selected | Disabled | Error |
|-----------|---------|-------|-----------------|----------|-------|
| **Provider Card** | Border: subtle | Border: primary, lift effect | Background: gradient tint | N/A | N/A |
| **Model Card** | Border: 2px subtle | Border: primary, lift | Border: primary, background: gradient, check icon | N/A | N/A |
| **Primary Button** | Background: primary-500 | Background: primary-600, lift | Lift effect | Opacity: 0.5, no hover | N/A |
| **Secondary Button** | Border: subtle, transparent bg | Border: secondary, bg: hover | N/A | N/A | N/A |
| **Input Field** | Border: primary | Border: secondary | Border: focus, glow shadow | N/A | Border: error |

### Validation States

| State | Icon | Background | Border | Text Color |
|-------|------|------------|--------|------------|
| **Info** | loader-2 (spinning) | info-light | info (30%) | info |
| **Success** | check-circle | success-light | success (30%) | success |
| **Error** | alert-circle | error-light | error (30%) | error |

### Configuration States

| State | Indicator | Description |
|-------|-----------|-------------|
| **Not Configured** | Gray dot, "Click to configure →" | Provider exists but no API key saved |
| **Configured** | Green checkmark, "✓ Configured" | API key saved and validated |
| **Validating** | Spinner, "Validating..." | API key validation in progress |
| **Error** | Alert icon, error message | Validation failed |

## Visual Design Specifications

### Colors

| Element | Color Token | Usage |
|---------|-------------|-------|
| **Primary Action** | `--color-primary-500` (#6366f1) | Primary buttons, selection indicators |
| **Provider - Anthropic** | `--color-provider-anthropic` (#d97706) | Anthropic provider branding |
| **Provider - OpenAI** | `--color-provider-openai` (#10b981) | OpenAI provider branding |
| **Provider - Google** | `--color-provider-google` (#4285f4) | Google provider branding |
| **Provider - Custom** | Gradient (#6366f1 to #8b5cf6) | Custom provider branding |
| **Background** | `--color-bg-secondary` (#18181b) | Modal backgrounds |
| **Surface** | `--color-surface` (#27272a) | Card and input backgrounds |

### Typography

| Element | Font Size | Weight | Line Height |
|---------|-----------|--------|-------------|
| **Modal Title** | `--text-xl` (20px) | Semibold (600) | Tight (1.25) |
| **Section Header** | `--text-sm` (14px) | Semibold (600) | Normal (1.5) |
| **Body Text** | `--text-base` (16px) | Normal (400) | Normal (1.5) |
| **Helper Text** | `--text-xs` (12px) | Normal (400) | Normal (1.5) |
| **Button Text** | `--text-sm` (14px) | Medium (500) | Normal (1.5) |

### Spacing

| Context | Spacing Token | Value |
|---------|---------------|-------|
| **Modal Padding** | `--space-5` | 16px |
| **Form Group Gap** | `--space-4` | 16px |
| **Card Gap** | `--space-4` | 16px |
| **Button Gap** | `--space-2` | 8px |
| **Text Gap** | `--space-2` | 8px |

### Borders and Radius

| Element | Border Radius | Border Width |
|---------|---------------|--------------|
| **Modal** | `--radius-2xl` (16px) | 1px (subtle) |
| **Card** | `--radius-xl` (12px) | 2px (primary when selected) |
| **Button** | `--radius-lg` (8px) | 1px (secondary) or none (primary) |
| **Input** | `--radius-lg` (8px) | 1px (primary) |

## Accessibility Requirements

### Keyboard Navigation

- **Tab Order**: Logical flow through interactive elements
- **Focus Indicators**: Visible focus ring on all interactive elements (`:focus-visible`)
- **Shortcuts**: ESC to close modals, Enter to submit forms
- **Arrow Navigation**: Not applicable (no grid navigation required)

### Screen Reader Support

- **ARIA Labels**: All icons have accessible labels or text alternatives
- **Live Regions**: Validation messages use appropriate ARIA live regions
- **Role Attributes**: Modals use `role="dialog"`, buttons use `role="button"`
- **State Announcements**: Configuration status changes are announced

### Color Contrast

- **Text**: Minimum 4.5:1 contrast ratio against background (WCAG AA)
- **Icons**: Minimum 3:1 contrast ratio for graphical elements
- **Focus Indicators**: Minimum 3:1 contrast ratio

### Error Handling

- **Validation Messages**: Clear, specific error text with actionable next steps
- **Recovery Path**: Clear way to fix errors and retry
- **Error Prevention**: Input validation before submission where possible

## States and Variants (Detailed)

### Provider Card States

```
┌─────────────────────────┐
│  [Icon]                 │  ← Default: Border subtle, no background
│  Anthropic              │
│  Claude Opus, Sonnet    │
│  ● Click to configure → │
└─────────────────────────┘

┌─────────────────────────┐
│ ╱╱[Icon]╱╱            │  ← Hover: Border primary, lift effect
│ ╱╱Anthropic╱╱          │
│ ╱╱Claude Opus, Sonnet╱╱ │
│ ╱╱● Click to configure →│
└─────────────────────────┘

┌─────────────────────────┐
│ ══[Icon]══             │  ← Configured: Border primary, tinted bg
│ ══Anthropic══          │
│ ══Claude Opus, Sonnet══│
│ ✓ ✓ Configured         │
└─────────────────────────┘
```

### Model Card States

```
┌─────────────────────┐
│ [AC]            [○] │  ← Default: 2px border, no fill check
│ Claude Opus         │
│ Anthropic           │
└─────────────────────┘

┌─────────────────────┐
│ ╱╱[AC]╱╱       ╱╱[○]│  ← Hover: Border color change, lift
│ ╱╱Claude Opus╱╱     │
│ ╱╱Anthropic╱╱       │
└─────────────────────┘

┌─────────────────────┐
│ ══[AC]══         ✓ │  ← Selected: Border primary, gradient bg, check icon
│ ══Claude Opus════   │
│ ══Anthropic══════   │
└─────────────────────┘
```

## Implementation Notes

### Technical Considerations

1. **API Key Storage**: Use platform-specific secure storage (OS keychain)
2. **Provider Validation**: Implement real API validation against provider endpoints
3. **Model Capabilities**: Maintain server-side mapping of model capabilities per provider
4. **State Persistence**: Save configuration to user preferences/settings storage
5. **Custom Provider HTTP**: Build HTTP client that adapts to OpenAI/Anthropic formats
6. **Icon System**: Use Lucide Icons library for consistent iconography

### Migration Notes

When implementing in the application:
- Convert prototype state management to application's state system (Svelte stores)
- Replace modal HTML with application's modal components
- Use CSS Modules for component-scoped styling
- Connect to backend services for API key validation and model discovery
- Implement proper error handling and loading states

---

## File Structure

```
config-flow/
├── index.html              # Main HTML (opens directly in browser)
├── README.md               # This documentation
├── css/
│   ├── design-system.css   # Design tokens and variables
│   ├── components.css      # Component-specific styles
│   └── animations.css      # Animation keyframes
└── js/
    └── state.js            # Alpine.js state management
```

## How to View

Simply open `index.html` in a modern web browser. No build process required.
