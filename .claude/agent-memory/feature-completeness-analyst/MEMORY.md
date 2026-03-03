# Feature Completeness Analyst Memory

## Project: Coder - AI Assistant Desktop Application

### Architecture Overview
- **Backend**: Rust with Tauri v2
- **Frontend**: Svelte + TypeScript
- **Database**: SQLite via rusqlite
- **Styling**: CSS modules with design system tokens

### Key File Locations
- **Backend Commands**: `/home/kassiomaia/devel/zk/coder/src-tauri/src/commands/`
- **Backend Storage**: `/home/kassiomaia/devel/zk/coder/src-tauri/src/storage/`
- **Frontend Stores**: `/home/kassiomaia/devel/zk/coder/src-ui/src/lib/stores/`
- **Frontend Components**: `/home/kassiomaia/devel/zk/coder/src-ui/src/lib/components/`
- **Design System**: `/home/kassiomaia/devel/zk/coder/src-ui/src/lib/styles/design-system.css`

### Observed Patterns
- One component per file with CSS modules
- Store pattern using Svelte writable/derived stores
- Tauri IPC commands for frontend-backend communication
- Optimistic UI updates with backend sync

### Critical Quality Gates
- Soft limit: 500 lines per file
- Hard limit: 800 lines per file
- Design system tokens must be used (no hardcoded values)
- Thread locking enforced at API level
