# Coder Product Documentation

This directory contains strategic product documents that guide the development and evolution of Coder.

## Document Index

### Product Requirements Documents (PRDs)

| Document | Status | Phase | Description |
|----------|--------|-------|-------------|
| [Search Feature](../features/search-prd.md) | Draft | Phase 1 | Full-text search across conversations and messages |
| [Message Editing](../features/message-editing-prd.md) | Draft | Phase 1 | Edit user messages with regeneration support |
| [Multi-Agent Platform](./multi-agent-platform-prd.md) | Vision | Long-term | Evolution from chat to multi-agent platform |

### User Journeys

| Document | Focus | Description |
|----------|-------|-------------|
| [First Chat](./user-journey-first-chat.md) | Onboarding | Complete user journey from installation to first successful chat |

---

## Strategic Roadmap

### Phase 1: Polish (Now - Q1 2026)

**Goal**: Perfect the core chat experience

| Feature | Priority | Status | PRD |
|---------|----------|--------|-----|
| Search | P0 | Planned | [Link](../features/search-prd.md) |
| Message Editing | P1 | Planned | [Link](../features/message-editing-prd.md) |
| Stream Cancellation Fix | P1 | Needed | - |
| Production Cleanup | P2 | Needed | - |

### Phase 2: Enhance (Q2 2026)

**Goal**: Add competitive features

| Feature | Priority | Status |
|---------|----------|--------|
| File Attachments | P1 | Planned |
| Archive System | P2 | Planned |
| Keyboard Shortcuts | P2 | Planned |

### Phase 3: Code Agent (Q2-Q3 2026)

**Goal**: Developer automation

| Feature | Priority | Status |
|---------|----------|--------|
| Workspace Detection | P0 | Planned |
| File Read/Write | P0 | Planned |
| Command Execution | P0 | Planned |
| Diff Viewer | P1 | Planned |

### Phase 4: Multi-Agent Platform (2027)

**Goal**: Professional AI assistants

| Agent Type | Target User |
|------------|-------------|
| Code Agent | Developers |
| Media Agent | Video Editors |
| Research Agent | Journalists |
| Domain Agents | Legal, Finance, etc. |

---

## Key Metrics

### Current Focus (Chat Quality)

1. **Time to First Message**: < 30 seconds from app open
2. **Streaming Smoothness**: First token in < 2 seconds
3. **Search Adoption**: Target 60% of users by week 4
4. **Day 2 Return Rate**: Target 40%

### Future Focus (Agent Effectiveness)

1. **Task Success Rate**: > 85%
2. **Human Intervention Rate**: < 20%
3. **Time Saved per Task**: > 50%

---

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-03-02 | Prioritize search over new features | Users cannot find past conversations - critical for retention |
| 2026-03-02 | Focus on chat before agents | Solid foundation required before automation |
| 2026-03-02 | Human-in-the-loop for all agent writes | Security and trust require explicit approval |

---

## Contributing

When creating new product documents:

1. **PRDs**: Use the template in `features/search-prd.md` as reference
2. **User Journeys**: Use `user-journey-first-chat.md` as template
3. **Keep updated**: Update status and last updated date when changing documents
4. **Link here**: Add new documents to the appropriate index table

---

**Last Updated**: 2026-03-02
