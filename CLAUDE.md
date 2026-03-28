# Coder - AI Assistant Desktop Application

## Project Overview

Coder is a modern desktop application built with Rust and Tauri
designed to help developers automate activities and write software more efficiently using AI agents.

### Design Thinking

1. **User-Centric Design**: Focus on creating an intuitive and seamless user experience that integrates AI capabilities without overwhelming the user.
2. **Performance**: Leverage Rust's performance and Tauri's native capabilities to ensure a responsive and efficient application.
3. **Modularity**: Design the application with a modular architecture to allow for easy maintenance and future feature expansion.
4. **Security**: Prioritize the security and privacy of user data, especially when handling code and AI interactions.
5. **Cross-Platform Compatibility**: Ensure the application runs smoothly on Windows, macOS, and Linux to reach a broad audience of developers.

### User Experience (UX)

- **Clean and Minimalist Interface**: Design a clean and minimalist interface that focuses on functionality while maintaining an aesthetically pleasing design.
- **Contextual AI Assistance**: Provide contextual AI assistance that is relevant to the user's current task, without being intrusive.
- **Customizability**: Allow users to customize their experience, such as choosing which AI agents to use and how they interact with the application.
- **Feedback Loop**: Implement a feedback mechanism for users to report issues or suggest improvements, fostering a community-driven development process.

### Key Features

- **AI Agent Integration**: Seamlessly integrate AI agents to assist with code generation, task automation, and productivity enhancements.
- **Code Generation**: Allow users to generate code snippets based on natural language descriptions.
- **Task Automation**: Enable users to automate repetitive development tasks using AI.
- **Productivity Tools**: Provide tools that enhance the developer workflow, such as code formatting, linting, and project management features.

### Goals

1. Provide amazing user experience through native desktop performance
2. Leverage desktop capabilities for enhanced productivity
3. Enable AI-powered automation for developers
4. Maintain security and privacy of user code

## Thread Locking

### Overview
Threads have two states: **unsolved** (open) and **resolved** (locked). When a thread is resolved, it enters a locked state that prevents further modifications.

### Thread States

1. **Unsolved (Open)**: The default state where threads can accept new messages and be actively modified.
2. **Resolved (Locked)**: A thread marked as complete that cannot accept new messages.

### Backend Behavior

The Rust backend enforces thread locking at the API level:

- **`send_message` command**: Validates that the target thread is not resolved before accepting any new message. Returns a `ThreadResolved` error if attempting to send to a resolved thread.
- **`create_thread_with_message` command**: Creates new threads (always in unsolved state).
- **`resolve_thread` command**: Toggles a thread's resolved status between unsolved and resolved.

The error message format when attempting to send to a resolved thread:
```
Thread '{thread_title}' is resolved and cannot accept new messages. Reopen the thread to continue the conversation.
```

### Frontend Behavior

The Svelte frontend reflects thread resolved state visually and functionally:

1. **ThreadPanel**: Displays a prominent warning banner when a thread is resolved, styled with warning colors from the design system.
2. **ChatInput**: Disables the text input when the thread is resolved, with visual feedback (warning background, reduced opacity).
3. **ChatArea**: Passes through the resolved state to the input component.

### User Flow

1. User can mark any thread as "resolved" via the resolve/reopen button in the ThreadItem.
2. Once resolved:
   - The thread appears in the "Resolved" section of the sidebar
   - A warning banner appears at the bottom of the thread view
   - The chat input is disabled with warning styling
3. User can "reopen" a resolved thread to continue the conversation:
   - Thread moves back to the "Open" section
   - Banner and input restrictions are removed
   - New messages can be sent again

### Design Tokens Used

For the resolved state UI, use these design system tokens:
- Background: `var(--color-warning-light)`
- Border: `var(--color-warning)`
- Text: `var(--color-warning)`
- Disabled opacity: `var(--opacity-disabled)`

## Tech Stack

- **Backend**: Rust
- **Frontend**: Svelte + TypeScript
- **Styling**: Custom CSS modules + Component library
- **Desktop Framework**: Tauri v2 (for building native Rust modules and API bindings for the frontend)

### Component Structure
- One component per file in `src-ui/src/lib/components/`
- Use CSS modules for component-scoped styles (e.g., `Button.module.css`)
- Keep components small and focused on single responsibility

### Naming Conventions
- Components: PascalCase (e.g., `ChatMessage.svelte`)
- CSS modules: Match component name (e.g., `ChatMessage.module.css`)
- Utility functions: camelCase (e.g., `formatTimestamp.ts`)

### Styling Approach
- Custom CSS modules for all component styles
- Global styles in `src-ui/src/lib/styles/global.css`
- CSS variables for theming (colors, spacing, typography)
- Desktop First Design: Prioritize desktop-specific design patterns and interactions, while ensuring responsiveness for smaller screens.
- Style Guide Adherence:
    -- **Consistency**: Ensure that all components and styles adhere to the design system and style guide defined in `src-ui/src/lib/styles/design-system.css` to maintain a cohesive look and feel across the application.
    -- **Unique Style**: The application should have a unique and visually appealing style that distinguishes it from generic applications or those built using component libraries without customization. The design system should be followed to ensure consistency and uniqueness in the application's appearance.
    -- **Design Tokens**: Colors, spacing, typography, and other design tokens must be defined in the design system and used consistently across the application to create a cohesive and visually appealing design.
    -- **Design System Awareness**: Before making any styling decisions, developers must be familiar with the design system and style guide to ensure that all styling choices align with the established guidelines. If there are any uncertainties about the design system, developers should consult with the design team to clarify any doubts before proceeding with styling decisions. This is crucial to avoid introducing "AI Slop" into the codebase and to maintain a consistent and visually appealing design throughout the application.
    -- **Colors, Spacing, Typography**: All design tokens such as colors, spacing, and typography must be defined in the design system and used consistently across the application to ensure a cohesive and visually appealing design. This includes adhering to the defined color palette, spacing guidelines, and typography styles to maintain a unified look and feel throughout the application.
    -- **Fonts**: The application should use a consistent set of fonts defined in the design system to ensure a cohesive and visually appealing typography across the application. This includes using the specified font families, sizes, weights, and styles as defined in the design system to maintain a unified and professional appearance throughout the application, avoid using generic fonts or fonts that do not align with the design system, and ensure that the typography contributes to the overall unique and visually appealing style of the application,
                  restrict use of fonts like Arial, Helvetica, Times New Roman, and other generic fonts that do not align with the design system. Instead, use the specified fonts defined in the design system to maintain a cohesive and visually appealing typography across the application.
- Before any styling decision is made, you must be aware of the design system and style guide defined in `src-ui/src/lib/styles/design-system.css` to ensure consistency across the application.
    a) In case the design system is not standardized, or if there are any doubts about the design system, you must reach out to the design team to clarify any doubts before making any styling decision.
    b) Do not generate "AI Slop" to the codebase, if you are not sure about the design system, or if there are any doubts about the design system, you must reach out to the design team to clarify any doubts before making any styling decision.
    c) The style of the application must be unique, consistent, and visually appealing, and it must not look like a generic application, or like an application that was built using a component library without any customization. The design system must be followed to ensure consistency across the application, and to ensure that the application has a unique and visually appealing style.
    b) Colors, spacing, typography, and other design tokens must be defined in the design system, and they must be used consistently across the application to ensure a cohesive and visually appealing design.

### CSS/Svelte Styling Guidelines

#### Prohibited Patterns - DO NOT USE

1. **Empty CSS rulesets**: Never create empty CSS rules. They cause Vite virtual CSS module loader to fail.
   ```css
   /* ❌ WRONG - Causes build errors */
   .sidebar-left {}

   /* ✅ CORRECT - Add a comment or remove the rule entirely */
   .sidebar-left {
       /* Styles inherited from parent */
   }
   /* Or simply remove if not needed */
   ```

2. **Invalid CSS properties**: Only use valid CSS properties. Data attributes cannot be set via CSS.
   ```css
   /* ❌ WRONG - Invalid syntax */
   :root {
       data-theme: 'light';
   }

   /* ✅ CORRECT - Use JavaScript to set data attributes */
   // In component: document.documentElement.setAttribute('data-theme', 'light');
   ```

3. **Undefined CSS variables**: Always verify variables exist in `design-system.css` before using them.
   ```css
   /* ❌ WRONG - Variable doesn't exist */
   transition: background var(--card-duration-hover) var(--card-ease-hover);

   /* ✅ CORRECT - Use defined variables from design-system.css */
   transition: background var(--transition-fast) var(--ease-out);
   ```

#### Required Verification Steps

Before committing any CSS changes:

1. **Check variable existence**: Search `src-ui/src/lib/styles/design-system.css` to verify the variable is defined
2. **Run type check**: Execute `npm run check` from `src-ui/` directory
3. **Run dev server**: Execute `npm run dev` to verify no CSS module loading errors
4. **Check for empty rulesets**: Run `grep -r "\. {}\|\.className {}" src-ui/src/lib/components`

#### Shared CSS Files - Usage Guidelines

The following shared CSS files exist in `src-ui/src/lib/styles/`:

| File | Purpose | When to Import |
|------|---------|----------------|
| `animations.css` | 30+ @keyframes (spin, fadeIn, slideIn, pulse) | Component needs animations |
| `scrollbar.css` | Unified scrollbar classes (.standard-scrollbar, .thin-scrollbar) | Component has scrollable content |
| `reduced-motion.css` | WCAG 2.1.2.3 compliant reduced motion | Component has animations |
| `design-system.css` | ALL design tokens (colors, spacing, typography) | Already imported globally - do NOT re-import |
| `global.css` | Global resets and base styles | Already imported in +layout.svelte |

**Import pattern**:
```css
/* In component .svelte files <style> block: */
@import '../../styles/animations.css';
@import '../../styles/scrollbar.css';

/* NEVER import design-system.css or global.css - already loaded globally */
```

#### Component CSS Best Practices

1. **Prefer component-scoped styles**: Use `<style>` blocks in `.svelte` files, not external CSS modules unless necessary

2. **Design tokens reference**: All available tokens are in `design-system.css`. Common categories:
   - Colors: `--color-primary-*`, `--color-neutral-*`, `--color-text-*`, `--color-border-*`
   - Spacing: `--space-0` through `--space-24`
   - Transitions: `--transition-instant`, `--transition-fast`, `--transition-normal`, `--transition-slow`
   - Easing: `--ease-out`, `--ease-in`, `--ease-in-out`
   - Radius: `--radius-sm`, `--radius-md`, `--radius-lg`, `--radius-xl`, `--radius-full`

3. **When in doubt, check first**: If unsure about a CSS variable, search the design-system.css file:
   ```bash
   grep -r "variable-name" src-ui/src/lib/styles/design-system.css
   ```

#### Common Mistakes and Solutions

| Mistake | Symptom | Solution |
|---------|---------|----------|
| Empty ruleset `{}` | `vite-plugin-svelte:load` failed | Add comment or remove rule |
| Undefined variable | Style not applied, console warning | Verify in design-system.css |
| Invalid CSS syntax | Build fails, parsing errors | Use valid CSS properties only |
| Duplicate @import | Circular dependency warnings | Don't import global files already loaded |

## Development Guidelines

### Code Style (Rust)
- Follow Rust standard naming conventions (snake_case for functions/variables, PascalCase for types)
- Use `clippy` for linting
- Write unit tests for core functionality

### Code Style (TypeScript/Svelte)
- Use TypeScript strict mode
- Follow ESLint and Prettier configurations
- Prefer composition over inheritance

### Architecture
- Modular design with clear separation of concerns
- Async/await for I/O operations
- Leverage Rust's type system for safety

### Quality Assurance
- Write unit tests for Rust backend and TypeScript frontend
- Use integration tests for critical user flows
- Regularly run linters and formatters to maintain code quality
- Small files with focused functionality (Any file in the project must be a soft limit of 500 lines of code, and hard limit of 800 lines of code)
    a) If in the process of Development any file exceeds 500 lines of code, it must be refactored into smaller files with focused functionality.
       This is to ensure maintainability and readability of the codebase.
- **Git**: Use Git for version control, with clear commit messages and regular commits to track changes effectively. Follow a branching strategy (e.g., Git Flow) to manage feature development, bug fixes, and releases in an organized manner.
- **Changelog**: Maintain a changelog to document significant changes, features, and bug fixes in the project. This helps in tracking the evolution of the project and provides transparency to users and contributors about what has changed in each version.

## Build Commands

```bash
# Rust backend
cargo build              # Build the project
cargo test               # Run tests
cargo clippy             # Run linter

# Frontend (run from src-ui/)
npm install              # Install dependencies
npm run dev              # Development server
npm run build            # Production build
npm run check            # Type checking

# Tauri (run from root)
cargo tauri dev          # Run in development mode
cargo tauri build        # Build for production
```

## Dependencies

### Rust
- `tauri` - Desktop application framework for building native Rust modules and API bindings for the frontend
- `tokio` - Async runtime
- `serde` - Serialization/deserialization
- `anyhow` - Error handling

### Frontend
- `svelte` - UI framework
- `@sveltejs/kit` - SvelteKit framework
- `typescript` - Type safety
- `vite` - Build tool


## Development Process

### General guidelines

- **Branching**: Use feature branches for new features and bug fixes. Merge to main branch only after code review and testing.
- **Plan**: Before starting development, create a plan outlining the feature or bug fix, including the expected outcome and any necessary design considerations. This helps ensure that development is focused and aligned with project goals.
- **Review**: Ensure the plan is good to go or if there are any doubts about the plan, reach out to the team to clarify any doubts before starting development. This is crucial to avoid misalignment and to ensure that the development process is efficient and effective.
- **Testing**: Write tests for new features and bug fixes to ensure code quality and prevent regressions. This includes unit tests for individual functions and integration tests for critical user flows.
- **Documentation**: Document new features and any significant changes to the codebase to help other developers understand the purpose and functionality of the code. This includes inline comments, README updates, and any relevant documentation files.

## New Feature Development

- **Feature Planning**: Before starting development on a new feature, create a detailed plan that outlines the feature's functionality, user experience, and any necessary design considerations. This plan should be reviewed and approved by the team to ensure alignment with project goals and to avoid any misunderstandings during development.
- **Design Collaboration**: If the new feature involves significant UI changes or additions, collaborate with the design team to create mockups or prototypes that align with the overall design system and style guide. This ensures that the new feature maintains a cohesive look and feel with the rest of the application.
- **Implementation**: Follow the established coding guidelines and best practices when implementing the new feature. This includes writing clean, maintainable code, adhering to the design system, and ensuring that the feature is well-tested.
- **Testing and Validation**: After implementing the new feature, thoroughly test it to ensure that it works as intended and does not introduce any regressions. This includes writing unit tests, integration tests, and performing manual testing to validate the user experience.

## Bug Fixes

- **Bug Identification**: Clearly identify and document the bug, including steps to reproduce, expected behavior, and actual behavior. This helps ensure that the bug is well-understood before starting the fix.
- **Plan the Fix**: Before implementing the bug fix, create a plan that outlines the
                    approach to fixing the bug, including any necessary code changes and testing strategies. This plan should be reviewed by the team to ensure that it is effective and does not introduce new issues.
- **Implement the Fix**: Follow the established coding guidelines and best practices when implementing the bug fix. This includes writing clean, maintainable code and ensuring that the fix is well-tested.
- **Testing and Validation**: After implementing the bug fix, thoroughly test it to ensure that it resolves the issue without introducing new problems. This includes writing unit tests, integration tests, and performing manual testing to validate that the bug has been successfully fixed and that the application continues to function as expected.

## Documents

- **Design System**: A comprehensive design system document that outlines the visual and interaction guidelines for the application, including colors, typography, spacing, and component styles. This document serves as a reference for developers and designers to ensure consistency across the application.
- **API Documentation**: Detailed documentation of the API endpoints and data structures used in the application, including any relevant information about authentication, request/response formats, and error handling. This documentation helps developers understand how to interact with the backend and ensures that API usage is consistent and well-understood.
- **Contribution Guidelines**: A document that outlines the process for contributing to the project, including guidelines for code style, testing, and documentation. This document helps ensure that contributions are consistent with the project's standards and that contributors understand the expectations for their contributions.
- **Changelog**: A document that tracks significant changes, features, and bug fixes in the project. This helps users and contributors understand the evolution of the project and provides transparency about what has changed in each version. The changelog should be updated regularly to reflect the latest changes in the project.

### How to store and maintain documents

- Store all project-related documents in a centralized location, such as a `docs/` directory in the project repository or a shared documentation platform (e.g., Confluence, Notion).
- Ensure that documents are organized in a clear and logical manner, with appropriate naming conventions and versioning to track changes over time.
- Regularly review and update documents to ensure that they remain accurate and relevant as the project evolves. This includes updating the design system, API documentation, contribution guidelines, and changelog as necessary to reflect any changes in the project.
- Encourage team members to refer to the documentation regularly and to contribute to its maintenance by providing feedback, suggesting improvements, and updating documents as needed. This helps ensure that the documentation remains a valuable resource for the team and contributes to the overall success of the project.


# IMPORTANT NOTE

- **Trust the process**: The development process outlined in this document is designed to ensure that the project is developed in a structured and efficient manner. It is important to trust the process and follow the guidelines to achieve the best results. If there are any doubts or uncertainties about the process, it is crucial to reach out to the team for clarification before proceeding with development. This helps avoid misalignment and ensures that everyone is on the same page, contributing to a successful development process and a high-quality final product.
- **Communication is key**: Effective communication within the team is essential for the success of the project. If there are any doubts, questions, or concerns about any aspect of the development process, it is important to communicate openly and promptly with the team. This includes reaching out for clarification on the design system, development guidelines, or any other aspect of the project to ensure that everyone has a clear understanding and can contribute effectively to the project's success.
- **Adherence to guidelines**: Following the established guidelines for code style, architecture, testing, and documentation is crucial for maintaining code quality and ensuring a cohesive and maintainable codebase. It is important to adhere to these guidelines consistently throughout the development process to avoid introducing technical debt and to ensure that the project remains organized and efficient as it evolves. If there are any uncertainties about the guidelines, it is important to seek clarification from the team before proceeding with development to ensure that everyone is aligned and contributing effectively to the project's success.
- **Provide feedback**: Constructive feedback is essential for continuous improvement and the overall success of the project. If you have any suggestions for improving the development process, code quality, or any other aspect of the project, it is important to share your feedback with the team. This helps foster a collaborative environment where everyone can contribute to making the project better and ensures that the development process remains effective and efficient as the project evolves.