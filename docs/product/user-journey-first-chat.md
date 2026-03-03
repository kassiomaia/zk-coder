# First Chat - User Journey Map

## Persona

**Alex - Senior Developer**
- 8 years of experience, works at a mid-sized tech company
- Uses AI tools daily (ChatGPT, GitHub Copilot, Cursor)
- Values efficiency and clean workflows
- Skeptical of tools that over-promise and under-deliver
- Prefers native desktop apps over Electron-based tools

## Scenario

Alex has just downloaded Coder after hearing about it from a colleague. They're looking for a native desktop AI assistant that doesn't feel sluggish. They open the app for the first time on a Saturday morning while working on a side project.

---

## Journey Stages

### Stage 1: Discovery & Installation

**Touchpoints:**
- GitHub release page / Website download
- Installation process (dmg, exe, AppImage)

**What Happens:**
1. Alex downloads the installer for their platform
2. Installs the application (drag to Applications, run .exe, etc.)
3. Opens the app for the first time

**User Thoughts:**
> "Let's see if this native app actually feels faster than Cursor..."

**Emotional State:** Curious, cautiously optimistic

**Pain Points:**
- Uncertainty about system requirements
- No clear indication of what to expect after install

**Opportunities:**
- Show a quick preview/screenshot during download
- First-run experience that sets expectations

---

### Stage 2: First Launch & Configuration

**Touchpoints:**
- App window appears
- Configuration/Setup screen
- API key input

**What Happens:**
1. App opens with a clean, minimal interface
2. Configuration wizard appears (if no API key stored)
3. Alex selects their preferred LLM provider
4. Enters their API key
5. Selects default model

**User Thoughts:**
> "Okay, this looks clean. I like that it's asking for my API key upfront - no subscription trap."

**Emotional State:** Interested, evaluating

**Pain Points:**
- May not have API key ready
- Uncertainty about which model to choose
- Security concerns about entering API key

**Opportunities:**
- "Get an API key" link that opens provider's page
- Model recommendations with use cases
- Explanation of where key is stored (keychain/local)

**Critical Moment:** API key entry is a trust barrier. Clear communication about security is essential.

---

### Stage 3: Empty State & First Message

**Touchpoints:**
- Empty conversation state
- Chat input with placeholder
- Sidebar (empty or with sample)

**What Happens:**
1. Configuration completes, main interface appears
2. Empty state with welcoming message/illustration
3. Alex sees the chat input with helpful placeholder
4. Types first message: "Help me understand the difference between async/await in Rust vs JavaScript"

**User Thoughts:**
> "I'll start with something I know to test the quality of responses..."

**Emotional State:** Testing, evaluating quality

**Pain Points:**
- Uncertainty about what the app is capable of
- Blank page syndrome
- No guidance on best practices for prompts

**Opportunities:**
- Example prompts related to development
- Quick-start suggestions based on time of day/recent files
- "What can I help you with?" with rotating examples

**Critical Moment:** First message determines if user believes the app is worth their time. Response quality and speed are paramount.

---

### Stage 4: First Response & Streaming

**Touchpoints:**
- Streaming response appears
- Markdown rendering
- Code syntax highlighting

**What Happens:**
1. Message sends, appears immediately in chat
2. Streaming indicator shows AI is "thinking"
3. Response streams in token by token
4. Code blocks render with syntax highlighting
5. Alex reads the response

**User Thoughts:**
> "Oh nice, the streaming is smooth. The code highlighting is clean. This response is actually pretty good."

**Emotional State:** Pleasantly surprised, engaged

**Pain Points:**
- If streaming is slow/jerky, perception of app quality drops
- Poor markdown rendering breaks immersion

**Opportunities:**
- Show estimated response time
- Smooth streaming animation
- Copy code button on code blocks

**Critical Moment:** Streaming quality is THE differentiator from web-based tools. Must feel instant and smooth.

---

### Stage 5: Follow-up & Thread Management

**Touchpoints:**
- Continued conversation
- Thread creation (if starting new topic)
- Sidebar with conversation list

**What Happens:**
1. Alex asks follow-up question about ownership in Rust
2. Response continues in same thread
3. Alex decides to start fresh topic, creates new thread
4. Notices threads appear in sidebar

**User Thoughts:**
> "I can organize my conversations! This is better than ChatGPT's flat list."

**Emotional State:** Satisfied, discovering features

**Pain Points:**
- How to create new thread vs continue current one may be unclear
- Naming/organizing threads requires effort

**Opportunities:**
- Auto-suggest thread names based on content
- Visual distinction between "continue" and "new thread"
- Thread templates for common workflows

---

### Stage 6: Resolution & Return

**Touchpoints:**
- Mark thread as resolved
- Close app
- Reopen later

**What Happens:**
1. Alex resolves their query, marks thread as "resolved"
2. Thread moves to resolved section
3. Closes app, satisfied with first experience
4. Returns the next day to continue working

**User Thoughts:**
> "This is actually useful. I'll keep it open in the background while I code."

**Emotional State:** Converted, planning to continue use

**Pain Points:**
- Forgetting which conversations had useful info
- Difficulty finding previous solutions

**Opportunities:**
- Search functionality (critical!)
- Favorites/bookmarks for important threads
- Quick-open shortcut from anywhere

---

## Emotional Arc

```
High │                     ╭──────────────────────╮
     │    ╭───────────────╯                      │
     │   ╱                                        ╰────
     │  ╱
Med  │ ╱
     │╱
Low  └─────────────────────────────────────────────────
      Install  Config  First   Response  Follow-up  Return
                       Message
```

**Key Emotional Beats:**
1. **Hope** at install (maybe this will be better)
2. **Trust test** at API key entry
3. **Evaluation** at first message
4. **Delight** at smooth streaming
5. **Discovery** at thread organization
6. **Commitment** at return visit

---

## Pain Points Summary

| Stage | Pain Point | Severity | Recommendation |
|-------|------------|----------|----------------|
| Installation | Unclear system requirements | Low | Add to download page |
| Configuration | API key security concerns | High | Clear security explanation |
| Configuration | Model selection confusion | Medium | Add recommendations |
| First Message | Blank page syndrome | Medium | Example prompts |
| First Message | Unclear capabilities | Medium | "What can I ask" section |
| Follow-up | Thread management confusion | Low | Better UI affordances |
| Return | Finding past conversations | High | **Implement search** |

---

## Opportunities Summary

| Opportunity | Impact | Effort | Priority |
|-------------|--------|--------|----------|
| Security explanation for API keys | High | Low | P0 |
| Example prompts in empty state | High | Low | P1 |
| Search functionality | Critical | Medium | P0 |
| Auto-suggest thread names | Medium | Medium | P2 |
| Copy code button | Medium | Low | P1 |
| Quick-open shortcut | High | Low | P1 |

---

## Key Metrics to Track

1. **Configuration Completion Rate**: % of users who complete API setup
2. **Time to First Message**: Seconds from app open to first send
3. **First Response Satisfaction**: Implicit (continue conversation) or explicit (thumbs up)
4. **Day 2 Return Rate**: % of users who return within 48 hours
5. **Thread Creation Rate**: Threads per session (indicates engagement)

---

## Recommendations

### Must Have (P0)

1. **Clear API Key Security Messaging**
   - "Your API key is stored locally in your system keychain"
   - "We never send your key to our servers"

2. **Search Implementation**
   - Critical for returning users to find past conversations
   - Keyboard shortcut for power users

3. **Empty State Examples**
   - 3-4 example prompts that showcase capabilities
   - Rotating examples based on time/context

### Should Have (P1)

1. **Copy Code Button**
   - One-click copy on all code blocks
   - Visual feedback on copy

2. **Model Recommendations**
   - "Recommended for coding tasks" badge
   - Brief description of tradeoffs

### Nice to Have (P2)

1. **Auto Thread Naming**
   - Generate title from first message
   - User can edit

2. **Welcome Tutorial**
   - Optional guided tour
   - Skip for experienced users

---

**Document Status**: Draft
**Author**: Product Team
**Created**: 2026-03-02
**Last Updated**: 2026-03-02
