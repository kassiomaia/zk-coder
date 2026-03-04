## Settings Module Design Guidelines

### 1. Overall Layout Architecture

**Two-Column Master-Detail Structure**
- **Left Sidebar (Navigation):** A fixed, collapsible navigation panel containing the primary module categories. This serves as the persistent navigation anchor across all settings views.
- **Right Content Area (Configuration):** A scrollable, dynamic content region that displays the configuration options for the selected module.

**Container Behavior**
- The layout maintains a maximum content width to ensure readability, centered within the available space for large screens.
- On smaller viewports or when the window is resized, the sidebar may collapse into a hamburger menu or overlay, while the content area adapts fluidly.

---

### 2. Left Sidebar Specifications

**Visual Hierarchy**
- **Header:** Displays "Settings" as the section title at the top, establishing context immediately.
- **Search Integration:** A search icon positioned in the header area allows users to quickly locate specific settings without navigating through categories.
- **Menu Toggle:** A hamburger menu icon provides access to global actions or view options.

**Navigation Items**
- Each navigation item consists of:
  - **Icon:** A simple, monochrome line icon representing the module (e.g., speaker for Sound, globe for Network)
  - **Label:** Clear, concise text label positioned to the right of the icon
  - **Active State:** The currently selected module receives distinct visual treatment (background highlight, subtle border, or color shift) to indicate selection
- **Grouping:** Related modules may be visually grouped with subtle spacing or dividers
- **Scroll Behavior:** The sidebar scrolls independently if the module list exceeds viewport height

**Module Examples from Reference:**
- Network
- Bluetooth
- Displays
- Sound (Active/Selected state)
- Power
- Multitasking
- Appearance
- Apps
- Notifications
- Search
- Online Accounts
- Sharing
- Wellbeing

---

### 3. Right Content Area Specifications

**Header Structure**
- **Title:** Large, prominent heading displaying the current module name (e.g., "Sound")
- **Close/Back Control:** A dismiss button (X icon) positioned in the top-right corner for modal contexts, or navigation controls if applicable
- **Horizontal Rule:** A subtle divider separates the header from the content below

**Content Organization**

The right panel organizes configuration options into **logical groupings** using **section cards**:

#### Section Card Pattern
- **Card Container:** Rounded rectangle with subtle background differentiation from the main content area
- **Section Title:** Optional, appears above the card when grouping multiple related settings
- **Internal Layout:** Vertical stack of related controls with consistent internal spacing

#### Control Row Pattern
Each configuration option follows a consistent row structure:
- **Full-Width Container:** Extends across the card width
- **Label Positioning:** Left-aligned, descriptive text explaining the setting
- **Control Positioning:** Right-aligned, the interactive element (dropdown, slider, toggle, button)
- **Divider Treatment:** Subtle horizontal lines separate rows within a card, except for the last item

---

### 4. Control Types and Specifications

#### Dropdown Selectors
- **Composition:** Icon + Selected Value + Dropdown Arrow
- **Behavior:** Click to reveal options in a dropdown menu overlay
- **Content:** Displays current selection with a trailing chevron/arrow indicator
- **Example:** "Output Device" showing "Headphones - H390 headset with microphone" with a headphone icon

#### Slider Controls
- **Composition:** Icon (optional) + Track + Draggable Handle + Visual Fill
- **Visual Design:**
  - Track: Neutral background color
  - Fill: Distinct accent color (red in reference) showing current value from left edge to handle position
  - Handle: Circular, elevated appearance for easy grabbing
- **Positioning:** Label left, slider right, aligned vertically
- **Examples:** "Output Volume" and "Input Volume"

#### Centered/Position Sliders
- **Specialized Slider:** For balance/pan controls or similar spatial adjustments
- **Visual Difference:** Handle starts at center position, fill extends from center to handle in both directions or shows neutral center point
- **Example:** "Balance" slider with centered default position

#### Action Buttons
- **Placement:** Right-aligned within row or card header
- **Style:** Filled or outlined button with clear action label
- **Example:** "Test..." button in the Output section header

#### Navigation Rows
- **Composition:** Label + Current Value/Status + Chevron Arrow
- **Behavior:** Click to navigate to sub-screen or expand additional options
- **Example:** "Volume Levels" and "Alert Sound" rows with right-pointing arrows

---

### 5. Section Grouping Strategy

**Primary Grouping: Functional Domains**
Group controls by their functional relationship rather than alphabetical or arbitrary ordering:

- **Output Section:** All controls related to audio output (device selection, volume, balance, testing)
- **Input Section:** All controls related to audio input (device selection, input volume)
- **Sounds Section:** System sound behaviors and alert configurations

**Visual Separation Between Sections**
- Clear vertical spacing between section cards (larger than internal card spacing)
- Section headers (if used) positioned above their respective cards with adequate margin

---

### 6. Iconography Guidelines

**Icon Placement**
- **Navigation Icons:** Leftmost position in sidebar items, fixed width for alignment
- **Control Icons:** Positioned inline with labels or within interactive elements to provide visual context
- **Action Icons:** Used sparingly for primary actions (close, search, menu)

**Icon Style**
- Monochrome, line-based icons for consistency
- Medium stroke weight for clarity at small sizes
- Semantic relevance to the function represented

---

### 7. Spacing and Layout Principles

**Density**
- Medium density: Comfortable spacing that prevents crowding while maintaining information density
- Adequate padding within cards (approximately 16-24px)
- Consistent gutter spacing between sidebar and content, and within content margins

**Alignment**
- Strict left alignment for all labels and section titles
- Right alignment for all interactive controls and values
- Baseline alignment between labels and their associated controls

**Responsive Considerations**
- Minimum widths for control elements to maintain usability
- Text truncation with ellipsis for long device names or labels
- Touch-friendly minimum sizes for interactive elements (44px recommended)

---

### 8. Interaction Patterns

**Selection Feedback**
- Immediate visual response on hover (subtle background change)
- Clear active/selected states for current navigation item
- Focus indicators for keyboard navigation accessibility

**State Persistence**
- Selected module remains highlighted in sidebar while its content displays
- User position within settings should be restorable on return

**Progressive Disclosure**
- Complex settings nested behind navigation rows (e.g., "Volume Levels" leading to detailed mixer)
- Secondary screens maintain breadcrumb or back navigation context

---

### 9. Accessibility Considerations

- **Keyboard Navigation:** Full tab order support through all interactive elements
- **Screen Reader Support:** Semantic HTML structure with proper ARIA labels
- **Visual Contrast:** Ensure all text and interactive elements meet WCAG contrast requirements (adaptable to your theme)
- **Focus Management:** Clear focus indicators, especially important for the sidebar navigation

---

### 10. Implementation Notes for Your Team

**What to Preserve:**
- The two-column master-detail architecture
- Card-based grouping of related controls
- Consistent row patterns with left-label/right-control alignment
- Section-based organization by functional domain
- Progressive disclosure for complex sub-settings

**What to Adapt:**
- **Color Palette:** Replace the dark theme with your project's color system, maintaining sufficient contrast ratios
- **Typography:** Apply your project's font stack and type scale, preserving the hierarchical relationships (large module titles, medium section labels, standard control text)
- **Border Radius:** Adapt card and button corner rounding to match your design system
- **Accent Colors:** Replace the red slider fill with your brand's accent color for active states

**Key Metric Targets:**
- Sidebar width: Fixed, approximately 240-280px
- Content max-width: Approximately 800-900px for optimal readability
- Row height: Minimum 48px for touch targets
- Card padding: 16-24px internal, 24-32px between cards