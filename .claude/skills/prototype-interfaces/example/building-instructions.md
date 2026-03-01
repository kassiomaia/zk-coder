# Building Prototypes - Step-by-Step Guide

This guide provides detailed instructions for creating prototypes from start to finish.

## Phase 1: Planning

### 1.1 Define the Scope

Before writing any code, clearly define:
- **What** feature/workflow is being prototyped
- **Why** it needs prototyping (validation, exploration, communication)
- **Who** will review it (product design, stakeholders, developers)
- **What success looks like** (what questions should it answer?)

### 1.2 Identify User Flows

Map out the user interactions:
1. Entry points - How does the user arrive at this feature?
2. Actions - What can the user do?
3. States - What are the different UI states?
4. Exit points - How does the user leave or complete the flow?

### 1.3 List Components

Create two lists:

**Components to Include:**
- Core UI elements directly related to the feature
- Supporting components that provide context

**Components to Exclude:**
- Unrelated features (use placeholders or simplified versions)
- Complex features not being tested (note why excluded)

## Phase 2: Setup

### 2.1 Create Folder Structure

```bash
# From project root
mkdir -p prototypes/<prototype-name>/css
mkdir -p prototypes/<prototype-name>/js
touch prototypes/<prototype-name>/index.html
touch prototypes/<prototype-name>/README.md
touch prototypes/<prototype-name>/css/design-system.css
touch prototypes/<prototype-name>/css/components.css
touch prototypes/<prototype-name>/css/animations.css
touch prototypes/<prototype-name>/js/state.js
```

### 2.2 Copy Design Tokens

Copy design system tokens from the project:

```bash
cp src-ui/src/lib/styles/design-system.css prototypes/<prototype-name>/css/
```

If the project doesn't have a consolidated design system file, copy relevant CSS custom properties for:
- Colors (primary, neutral, semantic)
- Typography (fonts, sizes, weights)
- Spacing scale
- Border radius
- Shadows
- Transitions

### 2.3 Create Base HTML

Start with the template from `folder-structure.md`. Ensure:
- Alpine.js is loaded (deferred)
- CSS files are in correct order
- Font imports are included

## Phase 3: Building

### 3.0 Code Separation Rules (MANDATORY)

**Before writing any code, understand the separation requirements:**

**CSS Separation:**
- ❌ NO `style="..."` attributes on any HTML element
- ❌ NO `<style>` blocks in `index.html`
- ✅ ALL styles in `css/components.css` or `css/animations.css`
- ✅ Use CSS classes for all styling

**JavaScript Separation:**
- ❌ NO `<script>` blocks containing logic in `index.html`
- ❌ NO inline event handlers like `onclick="function()"`
- ✅ ALL JavaScript logic in `js/` files
- ✅ Use Alpine.js directives (`@click`, `x-show`, etc.) which reference `js/state.js`

**Why this matters:** Clean separation makes prototypes easier to maintain, styles reusable, and HTML semantic. The prototype should demonstrate the feature, not the implementation technique.

### 3.1 Define State Structure

In `js/state.js`, define your state:

```javascript
/**
 * State Management for <Feature Name> Prototype
 */

function prototypeState() {
    return {
        // ========================================
        // CORE STATE
        // ========================================

        // Example: Modal visibility
        isOpen: false,

        // Example: Selected items
        selectedItems: [],

        // Example: Current step in a flow
        currentStep: 1,

        // Example: Loading states
        isLoading: false,

        // ========================================
        // DATA
        // ========================================

        // Example: List of items
        items: [
            { id: 1, name: 'Item 1' },
            { id: 2, name: 'Item 2' },
        ],

        // ========================================
        // COMPUTED PROPERTIES
        // ========================================

        get itemCount() {
            return this.items.length;
        },

        get hasSelection() {
            return this.selectedItems.length > 0;
        },

        // ========================================
        // METHODS
        // ========================================

        openModal() {
            this.isOpen = true;
        },

        closeModal() {
            this.isOpen = false;
        },

        toggleItem(id) {
            const index = this.selectedItems.indexOf(id);
            if (index > -1) {
                this.selectedItems.splice(index, 1);
            } else {
                this.selectedItems.push(id);
            }
        },

        nextStep() {
            if (this.currentStep < 3) {
                this.currentStep++;
            }
        },

        prevStep() {
            if (this.currentStep > 1) {
                this.currentStep--;
            }
        },

        // ========================================
        // INITIALIZATION
        // ========================================

        init() {
            // Any setup logic
            console.log('Prototype initialized');
        }
    };
}
```

### 3.2 Build HTML Structure

Use semantic HTML with Alpine.js directives:

```html
<!-- Container with state -->
<div class="prototype-container" x-data="prototypeState()">

    <!-- Navigation -->
    <nav class="nav">
        <button @click="currentStep = 1" :class="{ 'active': currentStep === 1 }">
            Step 1
        </button>
        <button @click="currentStep = 2" :class="{ 'active': currentStep === 2 }">
            Step 2
        </button>
    </nav>

    <!-- Content areas -->
    <section x-show="currentStep === 1" x-transition>
        <h2>Step 1 Content</h2>
        <!-- Content -->
    </section>

    <section x-show="currentStep === 2" x-transition>
        <h2>Step 2 Content</h2>
        <!-- Content -->
    </section>

    <!-- Modal example -->
    <div class="modal-overlay" x-show="isOpen" x-transition.opacity>
        <div class="modal" @click.away="closeModal">
            <h2>Modal Title</h2>
            <button @click="closeModal">Close</button>
        </div>
    </div>
</div>
```

### 3.3 Style Components

In `css/components.css`, use design tokens:

```css
/* ========================================
   COMPONENT NAME
   ======================================== */

.component-name {
    /* Use design tokens, not hardcoded values */
    background: var(--color-bg-secondary);
    border: 1px solid var(--color-border-primary);
    border-radius: var(--radius-lg);
    padding: var(--space-4);

    /* Transitions use design tokens */
    transition: all var(--transition-fast);
}

.component-name:hover {
    background: var(--color-surface-hover);
    border-color: var(--color-primary-500);
}

/* State variations */
.component-name.active {
    border-color: var(--color-primary-500);
    background: rgba(99, 102, 241, 0.1);
}

/* Child elements */
.component-name-header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-3);
}

.component-name-title {
    font-size: var(--text-lg);
    font-weight: var(--font-semibold);
    color: var(--color-text-primary);
}
```

### 3.4 Add Animations (Optional)

In `css/animations.css`:

```css
/* Keyframes */
@keyframes fadeIn {
    from {
        opacity: 0;
    }
    to {
        opacity: 1;
    }
}

@keyframes slideUp {
    from {
        opacity: 0;
        transform: translateY(20px);
    }
    to {
        opacity: 1;
        transform: translateY(0);
    }
}

@keyframes spin {
    from {
        transform: rotate(0deg);
    }
    to {
        transform: rotate(360deg);
    }
}

/* Animation utilities */
.animate-fade-in {
    animation: fadeIn var(--transition-normal) ease-out;
}

.animate-slide-up {
    animation: slideUp var(--transition-slow) ease-out;
}

.animate-spin {
    animation: spin 1s linear infinite;
}
```

## Phase 4: Documentation

### 4.1 Complete README.md

The README is a **feature design specification**, not prototype implementation documentation. It should be written for product designers and developers to understand the feature, not how the prototype works.

**Writing Guidelines:**
- Write at feature design level (WHAT and WHY), not implementation level (HOW)
- Describe behavior and appearance, not code or file structure
- Use language accessible to non-technical stakeholders
- Focus on user-facing functionality and design decisions
- Do NOT mention Alpine.js, JavaScript files, CSS files, or prototype structure

**Fill in all required sections** (see `folder-structure.md` for template):
- **Overview**: Business context and feature purpose
- **User Stories**: Who uses this and why
- **Functional Requirements**: What the system must do
- **UI Components**: Component hierarchy and responsibilities
- **Interaction Flows**: Step-by-step user journeys
- **Visual Design Specifications**: Layout, typography, colors, spacing
- **States and Variants**: Component states table
- **Accessibility Requirements**: Keyboard, screen reader, focus
- **Implementation Notes**: Technical considerations
- **Out of Scope**: What's explicitly not included

### 4.2 Add Code Comments

Comment complex logic in both HTML and JS:

```html
<!-- Step indicator: shows progress through the flow -->
<div class="step-indicator">
    <!-- ... -->
</div>
```

```javascript
// Validates the API key format
// Returns true if valid, false otherwise
validateApiKey(key) {
    return key && key.length >= 10;
},
```

## Phase 5: Testing

### 5.1 Browser Testing

Open `index.html` directly in browser and test:
- All interactions work as expected
- No console errors
- Responsive behavior (if applicable)
- Keyboard navigation
- Focus states

### 5.2 Cross-Browser

Test in at least:
- Chrome/Edge (Chromium)
- Firefox
- Safari (if on macOS)

### 5.3 Quality Checklist

Before considering complete:
- [ ] All user flows documented
- [ ] Design tokens used throughout
- [ ] No hardcoded colors/spacing
- [ ] Accessibility attributes present
- [ ] README fully completed
- [ ] Works without server (file:// protocol)

## Common Patterns

### Modal/Dialog

```html
<div class="modal-overlay"
     x-show="isOpen"
     x-transition:enter="transition ease-out duration-300"
     x-transition:enter-start="opacity-0"
     x-transition:enter-end="opacity-100"
     x-transition:leave="transition ease-in duration-200"
     x-transition:leave-start="opacity-100"
     x-transition:leave-end="opacity-0">

    <div class="modal"
         @click.away="isOpen = false"
         x-transition:enter="transition ease-out duration-300"
         x-transition:enter-start="opacity-0 transform translate-y-4"
         x-transition:enter-end="opacity-100 transform translate-y-0">
        <!-- Modal content -->
    </div>
</div>
```

### Tabs

```html
<div x-data="{ activeTab: 'tab1' }">
    <div class="tabs">
        <button :class="{ 'active': activeTab === 'tab1' }" @click="activeTab = 'tab1'">
            Tab 1
        </button>
        <button :class="{ 'active': activeTab === 'tab2' }" @click="activeTab = 'tab2'">
            Tab 2
        </button>
    </div>

    <div x-show="activeTab === 'tab1'">Content 1</div>
    <div x-show="activeTab === 'tab2'">Content 2</div>
</div>
```

### List with Selection

```html
<template x-for="item in items" :key="item.id">
    <div class="item"
         :class="{ 'selected': selectedItems.includes(item.id) }"
         @click="toggleItem(item.id)">

        <span x-text="item.name"></span>

        <div class="check" x-show="selectedItems.includes(item.id)">
            ✓
        </div>
    </div>
</template>
```

### Loading State

```html
<button :disabled="isLoading" @click="submit()">
    <span x-show="!isLoading">Submit</span>
    <span x-show="isLoading" class="animate-spin">Loading...</span>
</button>
```

## Tips

1. **Start simple** - Get basic structure working before adding polish
2. **Use console.log** - Debug state changes in browser console
3. **Test incrementally** - Check each feature as you build it
4. **Keep it isolated** - Don't reference external files outside prototype folder
5. **Document as you go** - Don't leave README until the end
6. **Use meaningful names** - Future you will thank present you
