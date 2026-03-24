# Code Review Checklist

## System Agents Implementation

---

## 1. General Requirements

### 1.1 Code Quality

- [ ] Code follows Rust style guidelines (4-space indentation)
- [ ] No `unwrap()` or `expect()` without good reason
- [ ] Proper error handling with meaningful error messages
- [ ] No TODO comments left in code
- [ ] Code is documented where complex

### 1.2 Security

- [ ] No hardcoded secrets or credentials
- [ ] Tool allowlist is enforced
- [ ] Command validation is implemented
- [ ] No injection vulnerabilities
- [ ] Proper input validation

### 1.3 Performance

- [ ] No blocking operations in async context
- [ ] Proper use of tokio (no blocking in spawn)
- [ ] Database connections are properly managed
- [ ] Memory usage is reasonable

---

## 2. Backend (Rust) Checklist

### 2.1 Module Structure

- [ ] `system/mod.rs` properly exports all modules
- [ ] No circular dependencies
- [ ] Modules are logically organized

### 2.2 Config (`config.rs`)

- [ ] `SystemConfig` has proper defaults
- [ ] All fields have appropriate types
- [ ] Implements `Default` trait correctly
- [ ] Serialization/deserialization works

### 2.3 Store (`store.rs`)

- [ ] Database schema is correct
- [ ] Migrations handle existing data
- [ ] Proper error handling
- [ ] Connections are thread-safe
- [ ] SQL queries are parameterized (no injection)

### 2.4 Scheduler (`scheduler.rs`)

- [ ] Cron expressions are validated
- [ ] Scheduler starts/stops properly
- [ ] Tasks are scheduled correctly
- [ ] Timezone handling is correct

### 2.5 Executor (`executor.rs`)

- [ ] Tool allowlist is enforced
- [ ] Command validation works
- [ ] Timeouts are implemented
- [ ] Errors are handled gracefully

### 2.6 Manager (`manager.rs`)

- [ ] Startup/shutdown is handled correctly
- [ ] Crash recovery works
- [ ] Retry logic is implemented
- [ ] Auto-pause works
- [ ] Proper logging

### 2.7 Tasks (`tasks/*.rs`)

- [ ] Task trait is implemented correctly
- [ ] Tasks use only allowed tools
- [ ] Error handling is appropriate
- [ ] Results are meaningful

### 2.8 Notifier (`notifier.rs`)

- [ ] Notifications are sent correctly
- [ ] Settings are respected
- [ ] Errors don't crash the app

### 2.9 Commands (`commands/system.rs`)

- [ ] All commands are registered
- [ ] Error handling returns proper errors
- [ ] State management is correct
- [ ] Commands are properly documented

---

## 3. Frontend (Svelte/TypeScript) Checklist

### 3.1 Store (`systemStore.ts`)

- [ ] Proper TypeScript types
- [ ] State management is reactive
- [ ] Error handling is present
- [ ] Loading states are handled

### 3.2 API (`api/system.ts`)

- [ ] Proper error handling
- [ ] Types match backend
- [ ] Commands are called correctly

### 3.3 Components (`SystemPanel.svelte`)

- [ ] Proper Svelte 5 patterns (using $state, $derived, $effect)
- [ ] Component is accessible (proper ARIA labels)
- [ ] Error states are handled
- [ ] Loading states are shown
- [ ] No memory leaks (proper cleanup in $effect)

### 3.4 Integration

- [ ] System tab is accessible
- [ ] Status bar button works
- [ ] Data flows correctly

---

## 4. Integration Checklist

### 4.1 lib.rs

- [ ] SystemManager is properly initialized
- [ ] Database is created
- [ ] Scheduler starts on app launch
- [ ] Shutdown is handled properly
- [ ] Commands are registered
- [ ] Error handling doesn't crash app

### 4.2 AgentState Changes

- [ ] ToolRegistry is properly exposed
- [ ] SessionStore is accessible
- [ ] No breaking changes to existing functionality

---

## 5. Testing Checklist

### 5.1 Unit Tests

- [ ] Core functions have tests
- [ ] Edge cases are covered
- [ ] Error cases are tested
- [ ] Tool allowlist is tested

### 5.2 Integration Tests

- [ ] Database operations work
- [ ] Scheduler starts/stops
- [ ] Tasks execute correctly
- [ ] Notifications work

### 5.3 Manual Testing

- [ ] Can run task manually
- [ ] Scheduler triggers tasks
- [ ] Notifications appear
- [ ] Dashboard shows correct data
- [ ] Crash recovery works

---

## 6. Documentation Checklist

### 6.1 Code Comments

- [ ] Complex logic is documented
- [ ] Public APIs have docs
- [ ] Error types are documented

### 6.2 Module Documentation

- [ ] `mod.rs` has module-level docs
- [ ] Purpose of each module is clear

---

## 7. Security Checklist

### 7.1 Tool Access

- [ ] Only allowed tools can execute
- [ ] Command validation works
- [ ] No write operations allowed
- [ ] Audit logging is in place

### 7.2 Data

- [ ] No sensitive data in logs
- [ ] Database is properly secured
- [ ] No data leakage

---

## 8. Common Issues to Watch For

### 8.1 Rust

- [ ] Missing `Send + Sync` on shared types
- [ ] Blocking calls in async context
- [ ] Missing error propagation
- [ ] Improper lifetime management

### 8.2 Svelte

- [ ] Memory leaks from subscriptions
- [ ] Reactive state not updating
- [ ] Missing error boundaries

### 8.3 Integration

- [ ] State not being shared correctly
- [ ] Commands not registered
- [ ] Database path issues

---

## 9. Review Process

### 9.1 Self-Review

Before submitting a PR, verify:

1. ✅ Code compiles without warnings
2. ✅ Tests pass
3. ✅ No debug print statements left
4. ✅ Commit messages are clear
5. ✅ Changes are atomic

### 9.2 Reviewer Checklist

1. ✅ Understand the changes
2. ✅ Verify security implications
3. ✅ Check for edge cases
4. ✅ Test locally if possible
5. ✅ Provide constructive feedback

---

## 10. Approval Criteria

A PR can be approved when:

- [ ] All checklist items are addressed
- [ ] At least one reviewer approves
- [ ] CI/CD passes
- [ ] No security concerns
- [ ] Documentation is updated if needed

---

## 11. Common PR Comments

| Comment | Action |
|---------|--------|
| "Use `?` operator" | Simplify error handling |
| "Add tests" | Cover edge cases |
| "Missing docs" | Add documentation |
| "Security concern" | Address immediately |
| "Can be simplified" | Refactor for clarity |
