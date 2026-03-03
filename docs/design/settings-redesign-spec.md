# GNOME-Style Centralized Settings Interface - UI Specification

## Executive Summary

This document specifies the complete UI/UX design for replacing the current modal-based configuration system with a modern GNOME-style centralized settings window featuring lateral sidebar navigation.

**Design Philosophy:** Refined minimalism with intentionality - clean, focused, and professional desktop UX that respects user attention while providing powerful configuration capabilities.

---

## 1. Visual Layout Mockups

### 1.1 Main Settings Window Structure

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Settings                                              ⚙  🔍  _  □  ✕  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────┐  ┌──────────────────────────────────────────────┐ │
│  │                     │  │                                              │ │
│  │  ○ OVERVIEW         │  │   ┌──────────────────────────────────────┐   │ │
│  │                     │  │   │  Configuration Status                │   │ │
│  │  ── AI PROVIDERS    │  │   │  ┌────────┐ ┌────────┐ ┌────────┐   │   │ │
│  │                     │  │   │  │  3/5   │ │ 12/18  │ │  ● OK  │   │   │ │
│  │  ○ Model Defaults   │  │   │  │Provider│ │ Models │ │ Health │   │   │ │
│  │                     │  │   │  └────────┘ └────────┘ └────────┘   │   │ │
│  │  ○ Advanced         │  │   └──────────────────────────────────────┘   │ │
│  │                     │  │                                              │ │
│  │  ○ About            │  │   ┌──────────────────────────────────────┐   │ │
│  │                     │  │   │  Quick Actions                        │   │ │
│  │                     │  │   │  ┌─────────────┐ ┌─────────────┐     │   │ │
│  │                     │  │   │  │ + Provider  │ │ Configure   │     │   │ │
│  │                     │  │   │  │             │ │ Models      │     │   │ │
│  │                     │  │   │  └─────────────┘ └─────────────┘     │   │ │
│  │                     │  │   └──────────────────────────────────────┘   │ │
│  │                     │  │                                              │ │
│  └─────────────────────┘  └──────────────────────────────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

Window Size: 1200x800 (optimal), 900x600 (minimum)
Sidebar Width: 260px (expanded), 72px (collapsed)
```

### 1.2 AI Providers Section

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                                                                              │
│  ┌─────────────────────┐  ┌──────────────────────────────────────────────┐  │
│  │                     │  │  AI Providers                    + Add Custom │  │
│  │  ○ Overview         │  │  ─────────────────────────────────────────── │  │
│  │                     │  │                                              │  │
│  │  ━━ AI PROVIDERS    │  │  ┌────────────────────────────────────────┐  │  │
│  │                     │  │  │ ⬡ Anthropic              ✓ Connected   │  │  │
│  │  ○ Model Defaults   │  │  │   Claude API · 14 models               │  │  │
│  │                     │  │  │                        [Edit] [Remove] │  │  │
│  │  ○ Advanced         │  │  └────────────────────────────────────────┘  │  │
│  │                     │  │                                              │  │
│  │  ○ About            │  │  ┌────────────────────────────────────────┐  │  │
│  │                     │  │  │ ◉ OpenAI                ✓ Connected    │  │  │
│  │                     │  │  │   GPT API · 8 models                   │  │  │
│  │                     │  │  │                        [Edit] [Remove] │  │  │
│  │                     │  │  └────────────────────────────────────────┘  │  │
│  │                     │  │                                              │  │
│  │                     │  │  ┌────────────────────────────────────────┐  │  │
│  │                     │  │  │ ◆ Google                ⚠ Not Config   │  │  │
│  │                     │  │  │   Gemini API · 0 models                │  │  │
│  │                     │  │  │                     [Configure]        │  │  │
│  │                     │  │  └────────────────────────────────────────┘  │  │
│  │                     │  │                                              │  │
│  └─────────────────────┘  └──────────────────────────────────────────────┘  │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 1.3 Provider Configuration Panel (In-Place)

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                                                                              │
│  ┌─────────────────────┐  ┌──────────────────────────────────────────────┐  │
│  │                     │  │  ← Back to Providers                         │  │
│  │  ○ Overview         │  │  ─────────────────────────────────────────── │  │
│  │                     │  │                                              │  │
│  │  ━━ AI PROVIDERS    │  │  ┌────────────────────────────────────────┐  │  │
│  │                     │  │  │                                        │  │  │
│  │  ○ Model Defaults   │  │  │  ⬡ Anthropic                          │  │  │
│  │                     │  │  │  Claude API Compatible                │  │  │
│  │  ○ Advanced         │  │  │                                        │  │  │
│  │                     │  │  │  API Key                              │  │  │
│  │  ○ About            │  │  │  ┌──────────────────────────────────┐ │  │  │
│  │                     │  │  │  │ •••••••••••••••••••••••  [👁]   │ │  │  │
│  │                     │  │  │  └──────────────────────────────────┘ │  │  │
│  │                     │  │  │                                        │  │  │
│  │                     │  │  │  Model Discovery                      │  │  │
│  │                     │  │  │  ○ Auto-fetch from API                │  │  │
│  │                     │  │  │  ● Manual model list                  │  │  │
│  │                     │  │  │  ○ Hybrid (both)                      │  │  │
│  │                     │  │  │                                        │  │  │
│  │                     │  │  │  Manual Models (14)        [+ Add]   │  │  │
│  │                     │  │  │  ┌──────────────────────────────────┐ │  │  │
│  │                     │  │  │  │ claude-3-opus-20240229     [✕]  │ │  │  │
│  │                     │  │  │  │ claude-3-sonnet-20240229    [✕]  │ │  │  │
│  │                     │  │  │  │ claude-3-haiku-20240307     [✕]  │ │  │  │
│  │                     │  │  │  └──────────────────────────────────┘ │  │  │
│  │                     │  │  │                                        │  │  │
│  │                     │  │  │  [Validate Connection]    [Save]      │  │  │
│  │                     │  │  └────────────────────────────────────────┘  │  │
│  │                     │  │                                              │  │
│  └─────────────────────┘  └──────────────────────────────────────────────┘  │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 1.4 Model Defaults Section

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                                                                              │
│  ┌─────────────────────┐  ┌──────────────────────────────────────────────┐  │
│  │                     │  │  Model Defaults                              │  │
│  │  ○ Overview         │  │  ─────────────────────────────────────────── │  │
│  │                     │  │  Select default models for each operation    │  │
│  │  ○ AI Providers     │  │                                              │  │
│  │                     │  │  ┌────────────────────────────────────────┐  │  │
│  │  ━━ MODEL DEFAULTS  │  │  │ 🔍 Search & Analysis                   │  │  │
│  │                     │  │  │ ────────────────────────────────────── │  │  │
│  │  ○ Advanced         │  │  │ Code understanding and search tasks    │  │  │
│  │                     │  │  │                                        │  │  │
│  │  ○ About            │  │  │ Selected: Claude 3 Opus               │  │  │
│  │                     │  │  │                                        │  │  │
│  │                     │  │  │ ┌──────────┐ ┌──────────┐ ┌──────────┐│  │  │
│  │                     │  │  │ │ Opus     │ │ Sonnet   │ │ GPT-4    ││  │  │
│  │                     │  │  │ │ ●        │ │ ○        │ │ ○        ││  │  │
│  │                     │  │  │ │ Selected │ │          │ │          ││  │  │
│  │                     │  │  │ └──────────┘ └──────────┘ └──────────┘│  │  │
│  │                     │  │  └────────────────────────────────────────┘  │  │
│  │                     │  │                                              │  │
│  │                     │  │  ┌────────────────────────────────────────┐  │  │
│  │                     │  │  │ </> Code Generation                    │  │  │
│  │                     │  │  │ ────────────────────────────────────── │  │  │
│  │                     │  │  │ Writing and editing code               │  │  │
│  │                     │  │  │                                        │  │  │
│  │                     │  │  │ Selected: Claude 3 Sonnet             │  │  │
│  │                     │  │  │ ...                                    │  │  │
│  │                     │  │  └────────────────────────────────────────┘  │  │
│  │                     │  │                                              │  │
│  └─────────────────────┘  └──────────────────────────────────────────────┘  │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 1.5 Advanced Section

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                                                                              │
│  ┌─────────────────────┐  ┌──────────────────────────────────────────────┐  │
│  │                     │  │  Advanced                                    │  │
│  │  ○ Overview         │  │  ─────────────────────────────────────────── │  │
│  │                     │  │                                              │  │
│  │  ○ AI Providers     │  │  ┌────────────────────────────────────────┐  │  │
│  │                     │  │  │ 📦 Configuration                       │  │  │
│  │  ○ Model Defaults   │  │  │ ────────────────────────────────────── │  │  │
│  │                     │  │  │                                        │  │  │
│  │  ━━ ADVANCED        │  │  │  [Export Configuration]                │  │  │
│  │                     │  │  │  Save all settings to a JSON file      │  │  │
│  │                     │  │  │                                        │  │  │
│  │  ○ About            │  │  │  [Import Configuration]                │  │  │
│  │                     │  │  │  Load settings from a JSON file        │  │  │
│  │                     │  │  │                                        │  │  │
│  │                     │  │  │  [Reset to Defaults]                   │  │  │
│  │                     │  │  │  Clear all configuration (⚠ careful)   │  │  │
│  │                     │  │  └────────────────────────────────────────┘  │  │
│  │                     │  │                                              │  │
│  │                     │  │  ┌────────────────────────────────────────┐  │  │
│  │                     │  │  │ 💓 System Health                       │  │  │
│  │                     │  │  │ ────────────────────────────────────── │  │  │
│  │                     │  │  │                                        │  │  │
│  │                     │  │  │  Backend:     ● Connected              │  │  │
│  │                     │  │  │  Database:    ● Healthy                │  │  │
│  │                     │  │  │  API Latency: 45ms                     │  │  │
│  │                     │  │  │                                        │  │  │
│  │                     │  │  │  [Refresh Status]                      │  │  │
│  │                     │  │  └────────────────────────────────────────┘  │  │
│  │                     │  │                                              │  │
│  └─────────────────────┘  └──────────────────────────────────────────────┘  │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 1.6 Global Search (Ctrl+K)

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                                                                              │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │  🔍  Search settings...                                           ✕   │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │  ⬡ Anthropic Provider                           AI Providers >       │  │
│  │     Configure Claude API access                                        │  │
│  ├───────────────────────────────────────────────────────────────────────┤  │
│  │  API Key Management                             Advanced >            │  │
│  │     View and manage stored API keys                                    │  │
│  ├───────────────────────────────────────────────────────────────────────┤  │
│  │  Model Defaults                                 Model Defaults >      │  │
│  │     Set default models for operations                                  │  │
│  ├───────────────────────────────────────────────────────────────────────┤  │
│  │  Export Configuration                           Advanced >            │  │
│  │     Save settings to file                                             │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ↑↓ Navigate  ↵ Select  Esc Close                                           │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Component Hierarchy

```
SettingsWindow (Tauri Window)
└── SettingsApp.svelte
    ├── SettingsHeader.svelte
    │   ├── SettingsBreadcrumb.svelte
    │   └── GlobalSearch.svelte
    │
    ├── SettingsSidebar.svelte
    │   ├── SidebarHeader.svelte (collapse toggle)
    │   ├── SidebarNav.svelte
    │   │   └── SidebarNavItem.svelte (reusable)
    │   └── SidebarFooter.svelte (version info)
    │
    └── SettingsContent.svelte (router outlet)
        ├── OverviewSection.svelte
        │   ├── StatusCard.svelte
        │   ├── QuickActionCard.svelte
        │   └── HealthIndicator.svelte
        │
        ├── ProvidersSection.svelte
        │   ├── ProviderList.svelte
        │   ├── ProviderListItem.svelte
        │   └── AddProviderButton.svelte
        │
        ├── ProviderConfigSection.svelte (sub-route)
        │   ├── ProviderHeader.svelte
        │   ├── ApiKeyInput.svelte
        │   ├── ModelDiscoverySelector.svelte
        │   └── ManualModelList.svelte
        │
        ├── AddProviderSection.svelte (sub-route)
        │   ├── ProviderForm.svelte
        │   └── ConnectionTest.svelte
        │
        ├── ModelsSection.svelte
        │   ├── OperationGroup.svelte
        │   ├── ModelSelector.svelte
        │   └── ModelCard.svelte (reuse existing)
        │
        ├── AdvancedSection.svelte
        │   ├── ConfigExport.svelte
        │   ├── ConfigImport.svelte
        │   ├── HealthMonitor.svelte
        │   └── DebugOptions.svelte
        │
        └── AboutSection.svelte
            ├── AppInfo.svelte
            ├── VersionInfo.svelte
            └── Credits.svelte
```

---

## 3. CSS Module Specifications

### 3.1 SettingsApp.module.css

```css
/**
 * Settings App - Root Container
 * GNOME-style window layout with sidebar navigation
 */

.container {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: var(--color-bg-primary);
  color: var(--color-text-primary);
}

.main {
  display: flex;
  flex: 1;
  overflow: hidden;
}

/* Focus ring for keyboard navigation */
.container:focus-within {
  outline: none;
}

/* Reduced motion preference */
@media (prefers-reduced-motion: reduce) {
  .container * {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }
}
```

### 3.2 SettingsSidebar.module.css

```css
/**
 * Settings Sidebar - Lateral Navigation
 * Collapsible sidebar with section navigation
 */

.sidebar {
  width: var(--settings-sidebar-width, 260px);
  min-width: var(--settings-sidebar-width, 260px);
  background: var(--color-bg-secondary);
  border-right: 1px solid var(--color-border-subtle);
  display: flex;
  flex-direction: column;
  transition: width var(--transition-normal), min-width var(--transition-normal);
}

.sidebar.collapsed {
  width: var(--settings-sidebar-collapsed, 72px);
  min-width: var(--settings-sidebar-collapsed, 72px);
}

/* Navigation container */
.nav {
  flex: 1;
  padding: var(--space-2);
  overflow-y: auto;
  overflow-x: hidden;
}

/* Navigation section groups */
.navGroup {
  margin-bottom: var(--space-4);
}

.navGroupTitle {
  font-size: var(--text-2xs);
  font-weight: var(--font-semibold);
  text-transform: uppercase;
  letter-spacing: var(--tracking-wider);
  color: var(--color-text-muted);
  padding: var(--space-2) var(--space-3);
  margin-bottom: var(--space-1);
}

.sidebar.collapsed .navGroupTitle {
  visibility: hidden;
  height: 0;
  padding: 0;
  margin: 0;
}

/* Navigation item */
.navItem {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-3);
  border-radius: var(--radius-md);
  color: var(--color-text-secondary);
  text-decoration: none;
  transition: all var(--transition-fast);
  cursor: pointer;
  border: none;
  background: transparent;
  width: 100%;
  text-align: left;
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
}

.navItem:hover {
  background: var(--color-interactive-hover);
  color: var(--color-text-primary);
}

.navItem:focus-visible {
  outline: 2px solid var(--color-primary-500);
  outline-offset: 2px;
}

.navItem.active {
  background: var(--color-interactive-selected);
  color: var(--color-accent-primary);
}

.navItem.active:hover {
  background: var(--color-interactive-selected-hover);
}

/* Navigation item icon */
.navItemIcon {
  flex-shrink: 0;
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* Navigation item text */
.navItemText {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.sidebar.collapsed .navItemText {
  display: none;
}

/* Active indicator dot */
.navItemIndicator {
  width: 6px;
  height: 6px;
  border-radius: var(--radius-full);
  background: var(--color-accent-primary);
  flex-shrink: 0;
}

.sidebar.collapsed .navItemIndicator {
  display: none;
}

/* Sidebar footer */
.footer {
  padding: var(--space-3);
  border-top: 1px solid var(--color-border-subtle);
  font-size: var(--text-xs);
  color: var(--color-text-muted);
  text-align: center;
}

.sidebar.collapsed .footer {
  display: none;
}
```

### 3.3 SettingsContent.module.css

```css
/**
 * Settings Content Area - Main Content Panel
 * Router outlet for settings sections
 */

.content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--color-bg-primary);
}

/* Content header with breadcrumb */
.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-4) var(--space-6);
  border-bottom: 1px solid var(--color-border-subtle);
  min-height: var(--header-height, 56px);
}

/* Breadcrumb navigation */
.breadcrumb {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--text-sm);
  color: var(--color-text-secondary);
}

.breadcrumbLink {
  color: var(--color-text-secondary);
  text-decoration: none;
  transition: color var(--transition-fast);
  background: none;
  border: none;
  padding: 0;
  cursor: pointer;
  font-size: inherit;
}

.breadcrumbLink:hover {
  color: var(--color-text-primary);
}

.breadcrumbLink:focus-visible {
  outline: 2px solid var(--color-primary-500);
  outline-offset: 2px;
  border-radius: var(--radius-sm);
}

.breadcrumbSeparator {
  color: var(--color-text-muted);
}

.breadcrumbCurrent {
  color: var(--color-text-primary);
  font-weight: var(--font-medium);
}

/* Scrollable content area */
.scrollArea {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-6);
}

/* Section transitions */
.section {
  animation: fadeSlideIn var(--transition-normal) ease-out;
}

@keyframes fadeSlideIn {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* Section container */
.sectionContainer {
  max-width: 900px;
  margin: 0 auto;
}

/* Section title */
.sectionTitle {
  font-size: var(--text-xl);
  font-weight: var(--font-semibold);
  color: var(--color-text-primary);
  margin-bottom: var(--space-1);
}

.sectionDescription {
  font-size: var(--text-sm);
  color: var(--color-text-secondary);
  margin-bottom: var(--space-6);
}
```

### 3.4 StatusCard.module.css

```css
/**
 * Status Card - Dashboard Status Indicator
 * Displays configuration status with visual feedback
 */

.card {
  background: var(--color-surface);
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-lg);
  padding: var(--space-5);
  transition: all var(--transition-fast);
}

.card:hover {
  border-color: var(--color-border-primary);
  background: var(--color-surface-hover);
}

.cardClickable {
  cursor: pointer;
}

.cardClickable:focus-visible {
  outline: 2px solid var(--color-primary-500);
  outline-offset: 2px;
}

/* Status indicator */
.statusDot {
  width: 8px;
  height: 8px;
  border-radius: var(--radius-full);
  flex-shrink: 0;
}

.statusDot.success {
  background: var(--color-success);
  box-shadow: 0 0 8px var(--color-success);
}

.statusDot.warning {
  background: var(--color-warning);
  box-shadow: 0 0 8px var(--color-warning);
}

.statusDot.error {
  background: var(--color-error);
  box-shadow: 0 0 8px var(--color-error);
}

.statusDot.idle {
  background: var(--color-text-muted);
}

/* Card content */
.cardHeader {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  margin-bottom: var(--space-3);
}

.cardIcon {
  width: 40px;
  height: 40px;
  border-radius: var(--radius-md);
  background: var(--color-interactive-selected);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-accent-primary);
}

.cardValue {
  font-size: var(--text-3xl);
  font-weight: var(--font-bold);
  color: var(--color-text-primary);
  line-height: var(--leading-tight);
}

.cardLabel {
  font-size: var(--text-sm);
  color: var(--color-text-secondary);
  margin-top: var(--space-1);
}
```

### 3.5 ProviderListItem.module.css

```css
/**
 * Provider List Item - Provider Configuration Entry
 * Displays provider status with actions
 */

.item {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  padding: var(--space-4);
  background: var(--color-surface);
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-lg);
  transition: all var(--transition-fast);
}

.item:hover {
  border-color: var(--color-border-primary);
  background: var(--color-surface-hover);
}

.item.configured {
  border-color: var(--color-success);
  background: var(--color-success-subtle);
}

.item.configured:hover {
  border-color: var(--color-success);
}

/* Provider icon */
.providerIcon {
  width: 48px;
  height: 48px;
  border-radius: var(--radius-md);
  background: var(--color-bg-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

/* Provider info */
.providerInfo {
  flex: 1;
  min-width: 0;
}

.providerName {
  font-size: var(--text-base);
  font-weight: var(--font-semibold);
  color: var(--color-text-primary);
  margin-bottom: var(--space-1);
}

.providerMeta {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--text-sm);
  color: var(--color-text-secondary);
}

.apiType {
  padding: var(--space-0-5) var(--space-2);
  background: var(--color-bg-tertiary);
  border-radius: var(--radius-sm);
  font-size: var(--text-xs);
  font-weight: var(--font-medium);
}

.modelCount {
  color: var(--color-text-tertiary);
}

/* Status badge */
.statusBadge {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1-5);
  padding: var(--space-1-5) var(--space-3);
  border-radius: var(--radius-full);
  font-size: var(--text-xs);
  font-weight: var(--font-medium);
}

.statusBadge.connected {
  background: var(--color-success-light);
  color: var(--color-success);
}

.statusBadge.notConfigured {
  background: var(--color-warning-light);
  color: var(--color-warning);
}

.statusBadge.error {
  background: var(--color-error-light);
  color: var(--color-error);
}

/* Actions */
.actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-shrink: 0;
}
```

### 3.6 GlobalSearch.module.css

```css
/**
 * Global Search - Settings Search Overlay
 * Keyboard-accessible search across all settings
 */

.overlay {
  position: fixed;
  inset: 0;
  background: var(--color-bg-overlay);
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding-top: 15vh;
  z-index: var(--z-modal);
  opacity: 0;
  visibility: hidden;
  transition: opacity var(--transition-fast), visibility var(--transition-fast);
}

.overlay.open {
  opacity: 1;
  visibility: visible;
}

/* Search container */
.container {
  width: 100%;
  max-width: 560px;
  background: var(--color-surface-elevated);
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-2xl);
  overflow: hidden;
  transform: translateY(-16px) scale(0.98);
  transition: transform var(--transition-normal);
}

.overlay.open .container {
  transform: translateY(0) scale(1);
}

/* Search input */
.searchInput {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-4);
  border-bottom: 1px solid var(--color-border-subtle);
}

.searchInput input {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  font-size: var(--text-base);
  color: var(--color-text-primary);
}

.searchInput input::placeholder {
  color: var(--color-text-muted);
}

.closeButton {
  padding: var(--space-1);
  border-radius: var(--radius-sm);
  background: transparent;
  border: none;
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.closeButton:hover {
  background: var(--color-interactive-hover);
  color: var(--color-text-primary);
}

/* Results */
.results {
  max-height: 400px;
  overflow-y: auto;
}

.resultItem {
  display: flex;
  align-items: flex-start;
  gap: var(--space-3);
  padding: var(--space-3) var(--space-4);
  cursor: pointer;
  transition: background var(--transition-fast);
}

.resultItem:hover,
.resultItem.selected {
  background: var(--color-interactive-hover);
}

.resultItem:focus-visible {
  outline: 2px solid var(--color-primary-500);
  outline-offset: -2px;
}

.resultIcon {
  width: 32px;
  height: 32px;
  border-radius: var(--radius-md);
  background: var(--color-bg-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  color: var(--color-text-secondary);
}

.resultContent {
  flex: 1;
  min-width: 0;
}

.resultTitle {
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  color: var(--color-text-primary);
  margin-bottom: var(--space-0-5);
}

.resultDescription {
  font-size: var(--text-xs);
  color: var(--color-text-secondary);
}

.resultSection {
  font-size: var(--text-xs);
  color: var(--color-text-muted);
  flex-shrink: 0;
}

/* Keyboard hints */
.keyboardHints {
  display: flex;
  gap: var(--space-4);
  padding: var(--space-3) var(--space-4);
  border-top: 1px solid var(--color-border-subtle);
  font-size: var(--text-xs);
  color: var(--color-text-muted);
}

.keyboardHint {
  display: flex;
  align-items: center;
  gap: var(--space-1-5);
}

.kbd {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 20px;
  padding: var(--space-0-5) var(--space-1);
  background: var(--color-bg-tertiary);
  border-radius: var(--radius-sm);
  font-size: var(--text-2xs);
  font-weight: var(--font-medium);
}
```

---

## 4. Component TypeScript Specifications

### 4.1 SettingsApp.svelte

```typescript
<script lang="ts">
  /**
   * SettingsApp - Root Settings Component
   * GNOME-style centralized configuration interface
   */
  import styles from './SettingsApp.module.css';
  import { settingsStore, canNavigateBack } from '$lib/stores/settingsStore';
  import { configStore } from '$lib/stores/configStore';
  import SettingsHeader from './SettingsHeader.svelte';
  import SettingsSidebar from './SettingsSidebar.svelte';
  import SettingsContent from './SettingsContent.svelte';
  import GlobalSearch from './GlobalSearch.svelte';

  interface Props {
    initialSection?: SettingsSection;
    onclose?: () => void;
  }

  let { initialSection = 'overview', onclose }: Props = $props();

  // Reactive state from store
  let currentSection = $derived($settingsStore.currentSection);
  let isSearchOpen = $derived($settingsStore.isSearchOpen);

  // Keyboard shortcuts
  function handleKeydown(e: KeyboardEvent) {
    // Global search shortcut (Ctrl+K or Cmd+K)
    if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
      e.preventDefault();
      settingsStore.openSearch();
    }

    // Close search on Escape
    if (e.key === 'Escape' && isSearchOpen) {
      settingsStore.closeSearch();
    }

    // Navigate back on Backspace (when not in input)
    if (e.key === 'Backspace' && !isInputFocused() && $canNavigateBack) {
      settingsStore.navigateBack();
    }
  }

  function isInputFocused(): boolean {
    const active = document.activeElement;
    return active instanceof HTMLInputElement ||
           active instanceof HTMLTextAreaElement;
  }

  // Initialize section
  $effect(() => {
    if (initialSection) {
      settingsStore.navigateTo(initialSection);
    }
  });
</script>
```

### 4.2 SettingsSidebar.svelte

```typescript
<script lang="ts">
  /**
   * SettingsSidebar - Lateral Navigation Component
   * Collapsible sidebar with section navigation
   */
  import styles from './SettingsSidebar.module.css';
  import { settingsStore } from '$lib/stores/settingsStore';
  import type { SettingsSection } from '$lib/types/settings';

  interface NavItem {
    id: SettingsSection;
    label: string;
    icon: Snippet;
    description: string;
  }

  interface Props {
    collapsed?: boolean;
    onToggleCollapse?: () => void;
  }

  let { collapsed = false, onToggleCollapse }: Props = $props();

  // Navigation items
  const navItems: NavItem[] = [
    { id: 'overview', label: 'Overview', description: 'Configuration status' },
    { id: 'providers', label: 'AI Providers', description: 'Manage AI providers' },
    { id: 'models', label: 'Model Defaults', description: 'Set default models' },
    { id: 'advanced', label: 'Advanced', description: 'Import, export, debug' },
    { id: 'about', label: 'About', description: 'Application info' }
  ];

  let currentSection = $derived($settingsStore.currentSection);
  let focusedIndex = $state<number | null>(null);

  // Keyboard navigation
  function handleKeydown(e: KeyboardEvent) {
    const items = navItems;

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      focusedIndex = focusedIndex === null ? 0 :
                      Math.min(focusedIndex + 1, items.length - 1);
    }

    if (e.key === 'ArrowUp') {
      e.preventDefault();
      focusedIndex = focusedIndex === null ? items.length - 1 :
                      Math.max(focusedIndex - 1, 0);
    }

    if (e.key === 'Enter' && focusedIndex !== null) {
      settingsStore.navigateTo(items[focusedIndex].id);
    }
  }

  function handleNavigate(section: SettingsSection) {
    settingsStore.navigateTo(section);
    focusedIndex = null;
  }
</script>
```

### 4.3 ProvidersSection.svelte

```typescript
<script lang="ts">
  /**
   * ProvidersSection - AI Provider Management
   * List, configure, and manage AI providers
   */
  import styles from './ProvidersSection.module.css';
  import { configStore } from '$lib/stores/configStore';
  import { settingsStore } from '$lib/stores/settingsStore';
  import ProviderListItem from './ProviderListItem.svelte';
  import Button from '../shared/Button.svelte';
  import type { Provider } from '$lib/types/config';

  interface Props {
    onConfigureProvider?: (providerId: string) => void;
    onAddCustom?: () => void;
  }

  let { onConfigureProvider, onAddCustom }: Props = $props();

  // Reactive state
  let providers = $derived($configStore.providers);
  let configuredCount = $derived(providers.filter(p => p.isConfigured).length);
  let isLoading = $derived($configStore.providersLoading);

  // Handlers
  async function handleRemoveProvider(providerId: string) {
    // Show confirmation dialog
    const confirmed = await showConfirmation({
      title: 'Remove Provider',
      message: 'Are you sure you want to remove this provider? API keys will be deleted.',
      confirmText: 'Remove',
      destructive: true
    });

    if (confirmed) {
      await configStore.removeProvider(providerId);
    }
  }

  function handleConfigure(providerId: string) {
    if (onConfigureProvider) {
      onConfigureProvider(providerId);
    } else {
      // Navigate to provider config sub-section
      settingsStore.navigateTo(`providers/${providerId}`);
    }
  }
</script>
```

---

## 5. Accessibility Requirements

### 5.1 Keyboard Navigation

| Action | Shortcut | Context |
|--------|----------|---------|
| Open global search | `Ctrl+K` / `Cmd+K` | Anywhere in settings |
| Close search/modal | `Escape` | Search open, sub-panel open |
| Navigate sidebar | `↑` / `↓` | Sidebar focused |
| Select sidebar item | `Enter` / `Space` | Sidebar item focused |
| Navigate back | `Backspace` | Not in input field |
| Toggle sidebar | `Ctrl+B` / `Cmd+B` | Anywhere in settings |
| Focus sidebar | `Alt+1` | Anywhere in settings |
| Focus content | `Alt+2` | Anywhere in settings |

### 5.2 ARIA Attributes

```html
<!-- Sidebar navigation -->
<nav
  role="navigation"
  aria-label="Settings navigation"
>
  <button
    role="menuitem"
    aria-current={isActive ? 'page' : undefined}
    aria-describedby="section-description-{id}"
  >
    {label}
  </button>
</nav>

<!-- Provider list -->
<ul role="list" aria-label="Configured providers">
  <li role="listitem" aria-setsize={total} aria-posinset={index}>
    {provider}
  </li>
</ul>

<!-- Status indicators -->
<span
  role="status"
  aria-live="polite"
  aria-label={`${count} of ${total} providers configured`}
>
  {count}/{total}
</span>

<!-- Global search -->
<dialog
  role="search"
  aria-label="Search settings"
  aria-modal="true"
>
  <input
    type="search"
    aria-label="Search settings"
    aria-describedby="search-hints"
  >
</dialog>
```

### 5.3 Focus Management

```typescript
// Focus management utilities
const focusManagement = {
  // Store last focused element before modal/search
  previousFocus: null as HTMLElement | null,

  trapFocus(container: HTMLElement) {
    const focusableElements = container.querySelectorAll(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    );
    const firstElement = focusableElements[0] as HTMLElement;
    const lastElement = focusableElements[focusableElements.length - 1] as HTMLElement;

    function handleTab(e: KeyboardEvent) {
      if (e.key !== 'Tab') return;

      if (e.shiftKey && document.activeElement === firstElement) {
        e.preventDefault();
        lastElement.focus();
      } else if (!e.shiftKey && document.activeElement === lastElement) {
        e.preventDefault();
        firstElement.focus();
      }
    }

    container.addEventListener('keydown', handleTab);
    return () => container.removeEventListener('keydown', handleTab);
  },

  saveFocus() {
    this.previousFocus = document.activeElement as HTMLElement;
  },

  restoreFocus() {
    if (this.previousFocus) {
      this.previousFocus.focus();
      this.previousFocus = null;
    }
  }
};
```

---

## 6. Transition and Animation Guidelines

### 6.1 Motion Principles

1. **Purposeful**: Animations guide attention and provide feedback
2. **Subtle**: Quick, unobtrusive transitions (150-300ms)
3. **Consistent**: Same easing curves throughout
4. **Accessible**: Respect `prefers-reduced-motion`

### 6.2 Transition Specifications

```css
/* Section transitions */
.section-enter {
  opacity: 0;
  transform: translateX(8px);
}

.section-enter-active {
  opacity: 1;
  transform: translateX(0);
  transition: opacity var(--transition-normal), transform var(--transition-normal);
}

.section-exit {
  opacity: 1;
  transform: translateX(0);
}

.section-exit-active {
  opacity: 0;
  transform: translateX(-8px);
  transition: opacity var(--transition-fast), transform var(--transition-fast);
}

/* Sidebar collapse */
.sidebar {
  transition: width var(--transition-slow), min-width var(--transition-slow);
}

.sidebar .navItemText {
  transition: opacity var(--transition-fast);
}

.sidebar.collapsed .navItemText {
  opacity: 0;
  transition: opacity var(--transition-instant);
}

/* Search overlay */
.search-overlay {
  opacity: 0;
  visibility: hidden;
  transition: opacity var(--transition-fast), visibility var(--transition-fast);
}

.search-overlay.open {
  opacity: 1;
  visibility: visible;
}

.search-container {
  transform: translateY(-16px) scale(0.98);
  transition: transform var(--transition-normal);
}

.search-overlay.open .search-container {
  transform: translateY(0) scale(1);
}

/* Status indicators */
.status-dot {
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.6; }
}

/* Reduced motion */
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
```

### 6.3 Loading States

```css
/* Skeleton loading */
.skeleton {
  background: linear-gradient(
    90deg,
    var(--color-bg-tertiary) 25%,
    var(--color-surface-hover) 50%,
    var(--color-bg-tertiary) 75%
  );
  background-size: 200% 100%;
  animation: skeleton-loading 1.5s infinite;
  border-radius: var(--radius-md);
}

@keyframes skeleton-loading {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}

/* Spinner */
.spinner {
  width: 20px;
  height: 20px;
  border: 2px solid var(--color-border-primary);
  border-top-color: var(--color-accent-primary);
  border-radius: var(--radius-full);
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
```

---

## 7. Design Token Usage Summary

| Token | Usage | Example |
|-------|-------|---------|
| `--color-bg-primary` | Main background | Settings window |
| `--color-bg-secondary` | Sidebar background | Sidebar panel |
| `--color-surface` | Card backgrounds | Provider cards, status cards |
| `--color-surface-hover` | Hover states | Card hover, list item hover |
| `--color-text-primary` | Primary text | Titles, labels |
| `--color-text-secondary` | Secondary text | Descriptions, meta info |
| `--color-text-muted` | Disabled text | Hints, placeholders |
| `--color-border-subtle` | Subtle borders | Section dividers |
| `--color-border-primary` | Primary borders | Card borders |
| `--color-accent-primary` | Primary accent | Active nav item, focus rings |
| `--color-success` | Success states | Connected badge |
| `--color-warning` | Warning states | Unconfigured badge |
| `--color-error` | Error states | Error messages |
| `--space-*` | Spacing | Margins, padding, gaps |
| `--radius-md` | Border radius | Cards, buttons |
| `--radius-lg` | Larger radius | Panels, modals |
| `--transition-fast` | Quick transitions | Hover states |
| `--transition-normal` | Standard transitions | Section changes |
| `--shadow-lg` | Shadows | Dropdowns, search overlay |

---

## 8. Implementation Checklist

### Foundation
- [ ] Create `SettingsApp.svelte` root component
- [ ] Create `settingsStore.ts` navigation store
- [ ] Create `SettingsSidebar.svelte` with navigation
- [ ] Create `SettingsContent.svelte` router outlet
- [ ] Implement section routing system

### Sections
- [ ] Create `OverviewSection.svelte` dashboard
- [ ] Migrate `ProvidersSection.svelte` from SettingsModal
- [ ] Migrate `ProviderConfigSection.svelte` from ConfigModal
- [ ] Migrate `ModelsSection.svelte` model defaults
- [ ] Create `AdvancedSection.svelte`
- [ ] Create `AboutSection.svelte`

### Features
- [ ] Implement `GlobalSearch.svelte` with keyboard shortcut
- [ ] Implement sidebar collapse/expand
- [ ] Add breadcrumb navigation
- [ ] Add keyboard navigation
- [ ] Add loading states
- [ ] Add error states

### Polish
- [ ] Verify design token usage
- [ ] Test accessibility (keyboard, screen reader)
- [ ] Test reduced motion preference
- [ ] Test responsive behavior
- [ ] Cross-platform testing

---

## 9. File Structure

```
src-ui/src/lib/
├── components/
│   └── settings/
│       ├── SettingsApp.svelte
│       ├── SettingsApp.module.css
│       ├── SettingsSidebar.svelte
│       ├── SettingsSidebar.module.css
│       ├── SettingsContent.svelte
│       ├── SettingsContent.module.css
│       ├── SettingsHeader.svelte
│       ├── SettingsHeader.module.css
│       ├── GlobalSearch.svelte
│       ├── GlobalSearch.module.css
│       │
│       ├── sections/
│       │   ├── OverviewSection.svelte
│       │   ├── OverviewSection.module.css
│       │   ├── ProvidersSection.svelte
│       │   ├── ProvidersSection.module.css
│       │   ├── ProviderConfigSection.svelte
│       │   ├── ProviderConfigSection.module.css
│       │   ├── AddProviderSection.svelte
│       │   ├── AddProviderSection.module.css
│       │   ├── ModelsSection.svelte
│       │   ├── ModelsSection.module.css
│       │   ├── AdvancedSection.svelte
│       │   ├── AdvancedSection.module.css
│       │   ├── AboutSection.svelte
│       │   └── AboutSection.module.css
│       │
│       └── shared/
│           ├── StatusCard.svelte
│           ├── StatusCard.module.css
│           ├── ProviderListItem.svelte
│           ├── ProviderListItem.module.css
│           ├── SectionHeader.svelte
│           └── SectionHeader.module.css
│
├── stores/
│   ├── settingsStore.ts
│   └── configStore.ts (existing - modify)
│
└── types/
    └── settings.ts
```

---

This specification provides a complete blueprint for implementing a GNOME-style centralized settings interface that is accessible, performant, and consistent with the Coder design system.
