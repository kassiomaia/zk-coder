# Frontend Implementation Tasks

> **Parent**: [Implementation Plan](./IMPLEMENTATION-PLAN.md)
> **Focus**: Svelte 5 + TypeScript changes in `src-ui/src/`

---

## Current Architecture Reference

**Existing Layout**: `ChatLayout.svelte` provides:

```
┌──────────────┬──────────────────────────────────────┐
│   Sidebar    │            Chat Area                 │
│ (280px)      │                                      │
│              │   ThreadPanel or ChatArea            │
│ Conversations│   - Messages                        │
│ Threads      │   - Input                           │
│              │   - Model Selector                   │
└──────────────┴──────────────────────────────────────┘
```

**Design System**: See `src-ui/src/lib/styles/design-system.css`

- CSS variables for colors, spacing, typography
- Dark/light theme support via `data-theme`
- Existing components: `Button`, `Input`, `Modal`, `Popover`, `Tooltip`

---

## §1. Agent Panel Components

### 1.1 Layout Integration

**Goal**: Add a collapsible right panel for agent UI.

**File**: `src-ui/src/lib/components/layout/AgentLayout.svelte` (new)

```svelte
<script lang="ts">
  import { fly, slide } from 'svelte/transition';
  import AgentPanel from '$lib/components/agent/AgentPanel.svelte';
  import type { AgentSession, PendingAction } from '$lib/types/agent';
  
  interface Props {
    session: AgentSession | null;
    pendingActions: PendingAction[];
    onApprove: (actionId: string) => void;
    onDeny: (actionId: string) => void;
    panelOpen: boolean;
    onTogglePanel: () => void;
  }
  
  let { session, pendingActions, onApprove, onDeny, panelOpen, onTogglePanel }: Props = $props();
</script>

<div class="agent-panel-wrapper" class:open={panelOpen}>
  {#if panelOpen && session}
    <AgentPanel
      {session}
      {pendingActions}
      {onApprove}
      {onDeny}
      onclose={onTogglePanel}
    />
  {:else if panelOpen}
    <div class="empty-panel">
      <p>No active agent session</p>
    </div>
  {/if}
</div>

<style>
  .agent-panel-wrapper {
    width: 0;
    transition: width var(--transition-normal);
    overflow: hidden;
    border-left: 1px solid var(--color-border-primary);
  }
  
  .agent-panel-wrapper.open {
    width: var(--agent-panel-width, 320px);
  }
  
  .empty-panel {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--color-text-muted);
  }
</style>
```

**Update**: `src-ui/src/lib/components/layout/ChatLayout.svelte`

Add agent panel slot:

```svelte
<div class={styles.main}>
  <!-- Sidebar -->
  <div class={styles.sidebarWrapper}>...</div>
  
  <!-- Chat content -->
  <div class={styles.content}>...</div>
  
  <!-- NEW: Agent Panel -->
  <slot name="agent-panel" />
</div>
```

### 1.2 Agent Panel Component

**File**: `src-ui/src/lib/components/agent/AgentPanel.svelte` (new)

```svelte
<script lang="ts">
  import { ChevronDown, ChevronRight, Terminal, FileText, CheckCircle } from 'lucide-svelte';
  import TaskList from './TaskList.svelte';
  import FileTree from './FileTree.svelte';
  import TerminalView from './TerminalView.svelte';
  import ApprovalDialog from './ApprovalDialog.svelte';
  import type { AgentSession, PendingAction, ExecutionStep } from '$lib/types/agent';
  
  interface Props {
    session: AgentSession;
    pendingActions: PendingAction[];
    onApprove: (actionId: string) => void;
    onDeny: (actionId: string) => void;
    onclose: () => void;
  }
  
  let { session, pendingActions, onApprove, onDeny, onclose }: Props = $props();
  
  // Collapsible sections
  let tasksExpanded = $state(true);
  let filesExpanded = $state(true);
  let terminalExpanded = $state(false);
  
  // Show approval dialog if pending actions exist
  let showApprovalDialog = $derived(pendingActions.length > 0);
  let currentApproval = $derived(pendingActions[0]);
</script>

<div class="agent-panel">
  <header class="panel-header">
    <h3>{session.agentName}</h3>
    <button class="close-btn" onclick={onclose} type="button">
      <ChevronDown size={16} />
    </button>
  </header>
  
  <!-- Session state indicator -->
  <div class="session-state" class:waiting={session.state.type === 'user_input_required'}>
    {#if session.state.type === 'running'}
      <span class="status-dot running"></span>
      <span>Executing step {session.state.current_step}/{session.state.max_steps}</span>
    {:else if session.state.type === 'user_input_required'}
      <span class="status-dot waiting"></span>
      <span>Waiting for approval</span>
    {:else if session.state.type === 'finished'}
      <span class="status-dot complete"></span>
      <span>Completed</span>
    {:else}
      <span class="status-dot"></span>
      <span>{session.state.type}</span>
    {/if}
  </div>
  
  <!-- Task list -->
  <section class="panel-section">
    <button 
      class="section-header" 
      onclick={() => tasksExpanded = !tasksExpanded}
      type="button"
    >
      {#if tasksExpanded}<ChevronDown size={14} />{:else}<ChevronRight size={14} />{/if}
      <span>Tasks</span>
      {#if session.tasks.length > 0}
        <span class="badge">{session.tasks.length}</span>
      {/if}
    </button>
    {#if tasksExpanded}
      <TaskList tasks={session.tasks} />
    {/if}
  </section>
  
  <!-- File tree -->
  <section class="panel-section">
    <button 
      class="section-header" 
      onclick={() => filesExpanded = !filesExpanded}
      type="button"
    >
      {#if filesExpanded}<ChevronDown size={14} />{:else}<ChevronRight size={14} />{/if}
      <FileText size={14} />
      <span>Files</span>
      {#if session.filesTouched.length > 0}
        <span class="badge">{session.filesTouched.length}</span>
      {/if}
    </button>
    {#if filesExpanded}
      <FileTree 
        files={session.filesTouched} 
        onfileclick={(path) => console.log('Open file:', path)}
      />
    {/if}
  </section>
  
  <!-- Terminal output -->
  <section class="panel-section terminal-section">
    <button 
      class="section-header" 
      onclick={() => terminalExpanded = !terminalExpanded}
      type="button"
    >
      {#if terminalExpanded}<ChevronDown size={14} />{:else}<ChevronRight size={14} />{/if}
      <Terminal size={14} />
      <span>Terminal</span>
    </button>
    {#if terminalExpanded}
      <TerminalView output={session.terminalOutput} />
    {/if}
  </section>
</div>

<!-- Approval Dialog (modal) -->
{#if showApprovalDialog && currentApproval}
  <ApprovalDialog
    action={currentApproval}
    onapprove={() => onApprove(currentApproval.id)}
    ondeny={() => onDeny(currentApproval.id)}
  />
{/if}

<style>
  .agent-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--color-bg-secondary);
    font-size: var(--text-sm);
  }
  
  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-3) var(--space-4);
    border-bottom: 1px solid var(--color-border-primary);
  }
  
  .panel-header h3 {
    font-size: var(--text-sm);
    font-weight: var(--font-medium);
    color: var(--color-text-primary);
  }
  
  .close-btn {
    padding: var(--space-1);
    border-radius: var(--radius-sm);
    color: var(--color-text-muted);
  }
  
  .close-btn:hover {
    background: var(--color-interactive-hover);
    color: var(--color-text-primary);
  }
  
  .session-state {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    background: var(--color-surface);
    font-size: var(--text-xs);
    color: var(--color-text-secondary);
  }
  
  .session-state.waiting {
    background: var(--color-warning-light);
    color: var(--color-warning-dark);
  }
  
  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-text-muted);
  }
  
  .status-dot.running {
    background: var(--color-primary-400);
    animation: pulse 1.5s ease-in-out infinite;
  }
  
  .status-dot.waiting {
    background: var(--color-warning);
  }
  
  .status-dot.complete {
    background: var(--color-success);
  }
  
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }
  
  .panel-section {
    border-bottom: 1px solid var(--color-border-subtle);
  }
  
  .section-header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-2) var(--space-4);
    text-align: left;
    font-size: var(--text-xs);
    font-weight: var(--font-medium);
    color: var(--color-text-secondary);
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
  }
  
  .section-header:hover {
    background: var(--color-interactive-hover);
  }
  
  .badge {
    margin-left: auto;
    padding: 0 var(--space-1-5);
    background: var(--color-neutral-700);
    border-radius: var(--radius-full);
    font-size: var(--text-2xs);
    color: var(--color-text-muted);
  }
  
  .terminal-section {
    flex: 1;
    min-height: 0;
  }
</style>
```

---

## §2. Task List Component

**File**: `src-ui/src/lib/components/agent/TaskList.svelte` (new)

```svelte
<script lang="ts">
  import { CheckCircle, Circle, Loader2, XCircle } from 'lucide-svelte';
  import type { AgentTask, TaskStatus } from '$lib/types/agent';
  
  interface Props {
    tasks: AgentTask[];
    ontaskclick?: (task: AgentTask) => void;
  }
  
  let { tasks, ontaskclick }: Props = $props();
  
  function getStatusIcon(status: TaskStatus) {
    switch (status) {
      case 'completed': return CheckCircle;
      case 'in_progress': return Loader2;
      case 'failed': return XCircle;
      default: return Circle;
    }
  }
  
  function getStatusClass(status: TaskStatus): string {
    switch (status) {
      case 'completed': return 'completed';
      case 'in_progress': return 'running';
      case 'failed': return 'failed';
      default: return 'pending';
    }
  }
</script>

<div class="task-list">
  {#each tasks as task (task.id)}
    <button 
      class="task-item"
      class:selected={task.selected}
      onclick={() => ontaskclick?.(task)}
      type="button"
    >
      <span class="status-icon {getStatusClass(task.status)}">
        {#if task.status === 'in_progress'}
          <Loader2 size={14} class="spinning" />
        {:else}
          <svelte:component this={getStatusIcon(task.status)} size={14} />
        {/if}
      </span>
      <span class="task-content">
        <span class="task-title">{task.title}</span>
        {#if task.description}
          <span class="task-description">{task.description}</span>
        {/if}
      </span>
    </button>
  {:else}
    <div class="empty-state">
      <p>No tasks yet</p>
    </div>
  {/each}
</div>

<style>
  .task-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-2) var(--space-4);
  }
  
  .task-item {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    padding: var(--space-2);
    border-radius: var(--radius-md);
    text-align: left;
    width: 100%;
  }
  
  .task-item:hover {
    background: var(--color-interactive-hover);
  }
  
  .task-item.selected {
    background: var(--color-interactive-selected);
  }
  
  .status-icon {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-text-muted);
  }
  
  .status-icon.completed {
    color: var(--color-success);
  }
  
  .status-icon.running {
    color: var(--color-primary-400);
  }
  
  .status-icon.failed {
    color: var(--color-error);
  }
  
  .spinning {
    animation: spin 1s linear infinite;
  }
  
  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
  
  .task-content {
    display: flex;
    flex-direction: column;
    gap: var(--space-0-5);
    min-width: 0;
  }
  
  .task-title {
    font-size: var(--text-sm);
    color: var(--color-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  
  .task-description {
    font-size: var(--text-xs);
    color: var(--color-text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  
  .empty-state {
    padding: var(--space-4);
    text-align: center;
    color: var(--color-text-muted);
    font-size: var(--text-xs);
  }
</style>
```

---

## §3. File Tree Component

**File**: `src-ui/src/lib/components/agent/FileTree.svelte` (new)

```svelte
<script lang="ts">
  import { File, FileCode, FileText, Folder, FolderOpen, ChevronRight, ChevronDown } from 'lucide-svelte';
  import type { TouchedFile } from '$lib/types/agent';
  
  interface Props {
    files: TouchedFile[];
    onfileclick?: (path: string) => void;
  }
  
  let { files, onfileclick }: Props = $props();
  
  // Group files by directory
  let fileTree = $derived(buildFileTree(files));
  
  function buildFileTree(files: TouchedFile[]): FileTreeNode {
    const root: FileTreeNode = { name: '', path: '', type: 'directory', children: [] };
    
    for (const file of files) {
      const parts = file.path.split('/');
      let current = root;
      
      for (let i = 0; i < parts.length; i++) {
        const part = parts[i];
        const isFile = i === parts.length - 1;
        
        let child = current.children?.find(c => c.name === part);
        
        if (!child) {
          child = {
            name: part,
            path: parts.slice(0, i + 1).join('/'),
            type: isFile ? 'file' : 'directory',
            children: isFile ? undefined : [],
            file: isFile ? file : undefined,
          };
          current.children?.push(child);
        }
        
        if (!isFile) {
          current = child;
        }
      }
    }
    
    return root;
  }
  
  function getFileIcon(file: TouchedFile) {
    const ext = file.path.split('.').pop()?.toLowerCase();
    switch (ext) {
      case 'ts':
      case 'js':
      case 'tsx':
      case 'jsx':
        return FileCode;
      case 'md':
      case 'txt':
        return FileText;
      default:
        return File;
    }
  }
</script>

<div class="file-tree">
  {#each fileTree.children || [] as node (node.path)}
    <TreeNode {node} {onfileclick} {getFileIcon} depth={0} />
  {/each}
</div>

<script lang="ts">
  interface FileTreeNode {
    name: string;
    path: string;
    type: 'file' | 'directory';
    children?: FileTreeNode[];
    file?: TouchedFile;
  }
</script>

{#snippet TreeNode(node: FileTreeNode, onfileclick: ((path: string) => void) | undefined, getFileIcon: (f: TouchedFile) => typeof File, depth: number)}
  {@const [expanded, setExpanded] = $state(node.type === 'directory' ? false : undefined)}
  
  {#if node.type === 'directory'}
    <button 
      class="tree-node directory"
      style="padding-left: {12 + depth * 12}px"
      onclick={() => setExpanded(!expanded)}
      type="button"
    >
      {#if expanded}
        <ChevronDown size={12} />
        <FolderOpen size={14} />
      {:else}
        <ChevronRight size={12} />
        <Folder size={14} />
      {/if}
      <span>{node.name}</span>
    </button>
    
    {#if expanded && node.children}
      {#each node.children as child (child.path)}
        {@render TreeNode(child, onfileclick, getFileIcon, depth + 1)}
      {/each}
    {/if}
  {:else if node.file}
    <button 
      class="tree-node file"
      class:modified={node.file.modified}
      style="padding-left: {12 + depth * 12}px"
      onclick={() => onfileclick?.(node.path)}
      type="button"
    >
      <svelte:component this={getFileIcon(node.file)} size={14} />
      <span>{node.name}</span>
      {#if node.file.modified}
        <span class="change-indicator">M</span>
      {/if}
    </button>
  {/if}
{/snippet}

<style>
  .file-tree {
    padding: var(--space-2) var(--space-4);
  }
  
  .tree-node {
    display: flex;
    align-items: center;
    gap: var(--space-1-5);
    width: 100%;
    padding: var(--space-1) var(--space-2);
    font-size: var(--text-xs);
    color: var(--color-text-secondary);
    border-radius: var(--radius-sm);
    text-align: left;
  }
  
  .tree-node:hover {
    background: var(--color-interactive-hover);
    color: var(--color-text-primary);
  }
  
  .tree-node.file.modified {
    color: var(--color-warning);
  }
  
  .change-indicator {
    margin-left: auto;
    font-size: var(--text-2xs);
    font-weight: var(--font-medium);
    color: var(--color-warning);
  }
</style>
```

---

## §4. Terminal View Component

**File**: `src-ui/src/lib/components/agent/TerminalView.svelte` (new)

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import type { TerminalLine } from '$lib/types/agent';
  
  interface Props {
    output: TerminalLine[];
  }
  
  let { output }: Props = $props();
  let container: HTMLDivElement;
  
  // Auto-scroll to bottom on new output
  $effect(() => {
    if (output.length > 0 && container) {
      container.scrollTop = container.scrollHeight;
    }
  });
</script>

<div class="terminal-view" bind:this={container}>
  {#each output as line (line.id)}
    <div class="terminal-line {line.type}">
      {#if line.timestamp}
        <span class="timestamp">{formatTime(line.timestamp)}</span>
      {/if}
      <span class="content">{line.content}</span>
    </div>
  {:else}
    <div class="empty-terminal">
      <span>Terminal output will appear here</span>
    </div>
  {/each}
</div>

<script lang="ts">
  function formatTime(date: Date): string {
    return date.toLocaleTimeString('en-US', { 
      hour: '2-digit', 
      minute: '2-digit',
      second: '2-digit',
      hour12: false 
    });
  }
</script>

<style>
  .terminal-view {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: var(--space-2);
    background: var(--color-neutral-950);
    border-radius: var(--radius-md);
    margin: 0 var(--space-4) var(--space-4);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    line-height: var(--leading-relaxed);
  }
  
  .terminal-line {
    display: flex;
    gap: var(--space-2);
    padding: var(--space-0-5) 0;
  }
  
  .terminal-line.stdout {
    color: var(--color-text-primary);
  }
  
  .terminal-line.stderr {
    color: var(--color-error);
  }
  
  .terminal-line.command {
    color: var(--color-primary-400);
  }
  
  .terminal-line.info {
    color: var(--color-text-muted);
    font-style: italic;
  }
  
  .timestamp {
    flex-shrink: 0;
    color: var(--color-text-muted);
    opacity: 0.6;
  }
  
  .content {
    white-space: pre-wrap;
    word-break: break-all;
  }
  
  .empty-terminal {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--color-text-muted);
    opacity: 0.5;
  }
</style>
```

---

## §5. Approval Dialog Component

**File**: `src-ui/src/lib/components/agent/ApprovalDialog.svelte` (new)

```svelte
<script lang="ts">
  import Modal from '$lib/components/shared/Modal.svelte';
  import Button from '$lib/components/shared/Button.svelte';
  import { AlertTriangle, FileEdit, Trash2, Terminal, Globe } from 'lucide-svelte';
  import type { PendingAction, ActionType } from '$lib/types/agent';
  
  interface Props {
    action: PendingAction;
    onapprove: () => void;
    ondeny: () => void;
  }
  
  let { action, onapprove, ondeny }: Props = $props();
  
  let isLoading = $state(false);
  
  async function handleApprove() {
    isLoading = true;
    try {
      await onapprove();
    } finally {
      isLoading = false;
    }
  }
  
  function getActionIcon(type: ActionType) {
    switch (type.type) {
      case 'write_file': return FileEdit;
      case 'delete_file': return Trash2;
      case 'execute_command': return Terminal;
      case 'web_request': return Globe;
      default: return AlertTriangle;
    }
  }
  
  function getActionLabel(type: ActionType): string {
    switch (type.type) {
      case 'write_file': return 'Write File';
      case 'delete_file': return 'Delete File';
      case 'execute_command': return 'Execute Command';
      case 'web_request': return 'Web Request';
      case 'read_file': return 'Read File';
      default: return 'Action';
    }
  }
</script>

<Modal open={true} onclose={ondeny}>
  <div class="approval-dialog">
    <div class="dialog-header">
      <div class="icon-wrapper warning">
        <svelte:component this={getActionIcon(action.action_type)} size={20} />
      </div>
      <div class="header-text">
        <h3>Allow {getActionLabel(action.action_type)}?</h3>
        <p class="tool-name">Tool: {action.tool_name}</p>
      </div>
    </div>
    
    <div class="dialog-content">
      <p class="description">{action.description}</p>
      
      {#if action.preview}
        <div class="preview-section">
          <div class="preview-header">
            <span>Preview</span>
          </div>
          <pre class="preview-content"><code>{action.preview.content}</code></pre>
        </div>
      {/if}
    </div>
    
    <div class="dialog-actions">
      <Button variant="ghost" onclick={ondeny} disabled={isLoading}>
        Deny
      </Button>
      <Button variant="primary" onclick={handleApprove} loading={isLoading}>
        Allow
      </Button>
    </div>
  </div>
</Modal>

<style>
  .approval-dialog {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-4);
    min-width: 400px;
    max-width: 600px;
  }
  
  .dialog-header {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
  
  .icon-wrapper {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    border-radius: var(--radius-lg);
    background: var(--color-warning-light);
    color: var(--color-warning);
  }
  
  .header-text h3 {
    font-size: var(--text-lg);
    font-weight: var(--font-semibold);
    color: var(--color-text-primary);
  }
  
  .tool-name {
    font-size: var(--text-xs);
    color: var(--color-text-muted);
    font-family: var(--font-mono);
  }
  
  .description {
    color: var(--color-text-secondary);
    line-height: var(--leading-relaxed);
  }
  
  .preview-section {
    margin-top: var(--space-2);
    border: 1px solid var(--color-border-primary);
    border-radius: var(--radius-md);
    overflow: hidden;
  }
  
  .preview-header {
    padding: var(--space-2) var(--space-3);
    background: var(--color-surface);
    font-size: var(--text-xs);
    font-weight: var(--font-medium);
    color: var(--color-text-secondary);
    border-bottom: 1px solid var(--color-border-subtle);
  }
  
  .preview-content {
    padding: var(--space-3);
    margin: 0;
    max-height: 200px;
    overflow: auto;
    background: var(--color-neutral-950);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--color-text-primary);
  }
  
  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    padding-top: var(--space-2);
  }
</style>
```

---

## §6. TypeScript Types

**File**: `src-ui/src/lib/types/agent.ts` (new)

```typescript
/**
 * Agent Types
 * Type definitions for agent orchestration in the frontend.
 */

// Session ID
export type SessionId = string;

// Agent ID
export type AgentId = string;

// Trust mode
export type TrustMode = 'trusted' | 'untrusted' | 'restricted';

// Task status
export type TaskStatus = 'pending' | 'in_progress' | 'completed' | 'failed' | 'cancelled';

// Permission level
export type PermissionLevel = 'auto_run' | 'approval_required' | 'disabled';

/**
 * Agent configuration
 */
export interface Agent {
  id: AgentId;
  name: string;
  description: string;
  systemPrompt: string;
  allowedTools: string[];
  defaultModelId: string;
  trustMode: TrustMode;
}

/**
 * Agent session (runtime instance)
 */
export interface AgentSession {
  id: SessionId;
  agentId: AgentId;
  agentName: string;
  conversationId: string;
  threadId: string;
  state: SessionState;
  workspaceRoot?: string;
  tasks: AgentTask[];
  filesTouched: TouchedFile[];
  terminalOutput: TerminalLine[];
  createdAt: Date;
  updatedAt: Date;
}

/**
 * Session lifecycle state
 */
export type SessionState =
  | { type: 'launching' }
  | { type: 'running'; currentStep: number; maxSteps: number }
  | { type: 'wait_for_request'; lastActionAt: Date }
  | { type: 'user_input_required'; pendingActions: PendingAction[] }
  | { type: 'finished'; completedAt: Date }
  | { type: 'cancelled'; reason: string }
  | { type: 'error'; message: string; recoverable: boolean };

/**
 * Task tracked by the agent
 */
export interface AgentTask {
  id: string;
  title: string;
  description?: string;
  status: TaskStatus;
  selected?: boolean;
}

/**
 * File touched by the agent
 */
export interface TouchedFile {
  path: string;
  modified: boolean;
  action: 'read' | 'write' | 'delete';
}

/**
 * Terminal output line
 */
export interface TerminalLine {
  id: string;
  type: 'stdout' | 'stderr' | 'command' | 'info';
  content: string;
  timestamp?: Date;
}

/**
 * Action awaiting user approval
 */
export interface PendingAction {
  id: string;
  toolName: string;
  actionType: ActionType;
  description: string;
  preview?: ActionPreview;
  arguments: Record<string, unknown>;
  createdAt: Date;
}

/**
 * Type of action being performed
 */
export type ActionType =
  | { type: 'read_file'; path: string }
  | { type: 'write_file'; path: string; sizeBytes: number }
  | { type: 'delete_file'; path: string }
  | { type: 'execute_command'; command: string }
  | { type: 'web_request'; url: string };

/**
 * Preview of action (for approval dialog)
 */
export interface ActionPreview {
  contentType: 'diff' | 'text' | 'json';
  content: string;
  language?: string;
}

/**
 * Agent event from backend
 */
export type AgentEvent =
  | { type: 'session_created'; sessionId: string; agentId: string }
  | { type: 'session_state_changed'; sessionId: string; state: SessionState }
  | { type: 'execution_started'; sessionId: string; requestId: string }
  | { type: 'execution_step'; sessionId: string; step: ExecutionStep }
  | { type: 'execution_complete'; sessionId: string; result: ExecutionResult }
  | { type: 'execution_error'; sessionId: string; error: string }
  | { type: 'approval_required'; sessionId: string; action: PendingAction }
  | { type: 'approval_approved'; sessionId: string; actionId: string }
  | { type: 'approval_denied'; sessionId: string; actionId: string };

/**
 * Execution step
 */
export interface ExecutionStep {
  stepNumber: number;
  action: StepAction;
  result?: StepResult;
  timestamp: Date;
}

export type StepAction =
  | { type: 'llm_request'; model: string }
  | { type: 'tool_call'; tool: string; arguments: Record<string, unknown> };

export interface StepResult {
  success: boolean;
  output: Record<string, unknown>;
  tokensUsed?: number;
  durationMs: number;
}

/**
 * Execution result
 */
export interface ExecutionResult {
  content: string;
  steps: ExecutionStep[];
  usage?: {
    inputTokens: number;
    outputTokens: number;
    totalTokens: number;
  };
}
```

---

## §7. Agent Store

**File**: `src-ui/src/lib/stores/agentStore.ts` (new)

```typescript
import { writable, derived, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { 
  AgentSession, 
  PendingAction, 
  AgentEvent,
  SessionState 
} from '$lib/types/agent';

interface AgentState {
  sessions: Map<string, AgentSession>;
  activeSessionId: string | null;
  pendingActions: PendingAction[];
  loading: boolean;
  error: string | null;
}

function createAgentStore() {
  const { subscribe, set, update } = writable<AgentState>({
    sessions: new Map(),
    activeSessionId: null,
    pendingActions: [],
    loading: false,
    error: null,
  });

  let unsubscribe: (() => void) | null = null;

  async function initialize() {
    // Listen for agent events from backend
    unsubscribe = await listen<AgentEvent>('agent:event', (event) => {
      handleAgentEvent(event.payload);
    });
  }

  function handleAgentEvent(event: AgentEvent) {
    update((state) => {
      switch (event.type) {
        case 'session_created':
          // Session will be added when we fetch it
          break;
        case 'session_state_changed':
          const session = state.sessions.get(event.sessionId);
          if (session) {
            session.state = event.state;
            state.sessions.set(event.sessionId, { ...session });
          }
          break;
        case 'approval_required':
          state.pendingActions.push(event.action);
          break;
        case 'approval_approved':
        case 'approval_denied':
          state.pendingActions = state.pendingActions.filter(
            (a) => a.id !== event.actionId
          );
          break;
      }
      return { ...state };
    });
  }

  async function createSession(
    agentId: string,
    conversationId: string,
    threadId: string,
    workspaceRoot?: string
  ): Promise<AgentSession> {
    update((s) => ({ ...s, loading: true, error: null }));

    try {
      const session = await invoke<AgentSession>('create_agent_session', {
        agentId,
        conversationId,
        threadId,
        workspaceRoot,
      });

      update((state) => {
        state.sessions.set(session.id, session);
        return {
          ...state,
          activeSessionId: session.id,
          loading: false,
        };
      });

      return session;
    } catch (error) {
      update((s) => ({ ...s, loading: false, error: String(error) }));
      throw error;
    }
  }

  async function sendRequest(sessionId: string, content: string): Promise<void> {
    await invoke('send_agent_request', { sessionId, content });
  }

  async function approveAction(sessionId: string, actionId: string): Promise<void> {
    await invoke('approve_action', { sessionId, actionId });
  }

  async function denyAction(sessionId: string, actionId: string): Promise<void> {
    await invoke('deny_action', { sessionId, actionId });
  }

  async function getPendingActions(sessionId: string): Promise<PendingAction[]> {
    return invoke('get_pending_actions', { sessionId });
  }

  async function cancelSession(sessionId: string): Promise<void> {
    await invoke('cancel_session', { sessionId });
    update((state) => {
      state.sessions.delete(sessionId);
      if (state.activeSessionId === sessionId) {
        state.activeSessionId = null;
      }
      return { ...state };
    });
  }

  function destroy() {
    unsubscribe?.();
  }

  return {
    subscribe,
    initialize,
    createSession,
    sendRequest,
    approveAction,
    denyAction,
    getPendingActions,
    cancelSession,
    destroy,
  };
}

export const agentStore = createAgentStore();

// Derived stores
export const activeSession = derived(agentStore, ($state) =>
  $state.activeSessionId ? $state.sessions.get($state.activeSessionId) : null
);

export const pendingActions = derived(agentStore, ($state) =>
  $state.pendingActions
);
```

---

---

## §8. Workspace Selection

### 8.1 Conversation Workspace UI

**Decision**: Each conversation has an associated workspace folder.

**UI Changes**:

1. **Conversation Creation**: Add optional workspace selector
2. **Conversation Settings**: Add workspace path field
3. **Conversation Header**: Show workspace indicator when set

**File**: `src-ui/src/lib/components/chat/sidebar/ConversationSidebar.svelte` (update)

Add workspace selector to new conversation flow:

```svelte
<!-- In create conversation handler -->
<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  
  async function handleCreateConversation() {
    // Optionally select workspace
    const workspace = await open({
      directory: true,
      multiple: false,
      title: 'Select Workspace Folder',
    });
    
    const conversation = await chatStore.createConversation(
      'New Conversation',
      workspace as string | undefined
    );
  }
</script>
```

### 8.2 Workspace Indicator

**File**: `src-ui/src/lib/components/chat/ChatArea.svelte` (update)

Show workspace in conversation header:

```svelte
{#if selectedConversation?.workspaceRoot}
  <div class="workspace-indicator">
    <Folder size={12} />
    <span>{getWorkspaceName(selectedConversation.workspaceRoot)}</span>
  </div>
{/if}
```

### 8.3 Update Conversation Type

**File**: `src-ui/src/lib/types/thread.ts` (update)

```typescript
export interface Conversation {
  id: string;
  title: string;
  createdAt: Date;
  updatedAt: Date;
  threads?: Thread[];
  settings: ConversationSettings;
  workspaceRoot?: string;  // NEW
}

export interface ConversationSettings {
  model: string;
  temperature: number;
  maxTokens: number;
  systemPrompt?: string;
  workspacePath?: string;  // NEW
}
```

### 8.4 Update Chat Store

**File**: `src-ui/src/lib/stores/chat.ts` (update)

Add workspace to create conversation:

```typescript
async createConversation(title: string, workspacePath?: string): Promise<Conversation | null> {
  const conversation = await invoke<Conversation>('create_conversation', {
    title,
    workspacePath,
  });
  // ... rest of logic
}

async setWorkspace(conversationId: string, workspacePath: string): Promise<void> {
  await invoke('set_conversation_workspace', { conversationId, workspacePath });
  // Update local state
}
```

---

## Task Checklist

### §1 Layout Integration
- [x] Create `AgentLayout.svelte` wrapper → Integrated directly into ChatLayout (valid simplification)
- [x] Update `ChatLayout.svelte` with agent panel slot
- [x] Add CSS variable `--agent-panel-width` (in design-system.css)
- [x] Add responsive behavior (hide on mobile)

### §2 Core Components
- [x] Create `AgentPanel.svelte` main container
- [x] Create `TaskList.svelte` with status icons and subtask support
- [x] Create `FileTree.svelte` with tree rendering and change indicators (M/A/D)
- [x] Create `TerminalView.svelte` with auto-scroll
- [x] Create `ApprovalDialog.svelte` modal with DiffViewer preview
- [x] Create `DiffViewer.svelte` (bonus - syntax-colored diff)
- [x] Create `WorkspaceSelector.svelte` (bonus - folder picker)
- [x] Create `SessionHistory.svelte` (bonus - view completed sessions)
- [x] Create `SessionTabs.svelte` (bonus - multiple sessions)
- [x] Create `AgentConfig.svelte` (bonus - agent selection modal)

### §3 Types and Store
- [x] Create `types/agent.ts` with all interfaces
- [x] Create `stores/agentStore.ts` with state management
- [x] Add event listener initialization
- [x] Export from `lib/index.ts`
- [x] Fix optional fields (`tasks`, `filesTouched`, `terminalOutput`) in AgentSession type

### §4 API Integration
- [x] Wire Tauri commands to store methods
- [x] Handle agent events from backend
- [x] Add error handling and loading states
- [x] Fix ExecutionComplete event to include conversationId/threadId (for page reload scenario)

### §5 Workspace Integration
- [x] Update `Conversation` type with `workspaceRoot`
- [x] Update `ConversationSettings` with `workspaceRoot`
- [x] Add workspace selector to agent panel empty state
- [x] Update chat store with `setConversationWorkspace` method
- [x] Add `setConversationWorkspace` API function
- [x] Wire workspace changes to sync both session AND conversation

### §6 Testing
- [ ] Unit tests for components
- [ ] Integration tests for store
- [ ] E2E test for approval flow
- [ ] E2E test for workspace selection

---

## Component Directory Structure

```
src-ui/src/lib/components/agent/
├── AgentPanel.svelte          # Main panel container ✅
├── TaskList.svelte            # Task list with status ✅
├── FileTree.svelte            # File tree view ✅
├── TerminalView.svelte        # Terminal output ✅
├── ApprovalDialog.svelte      # Approval modal ✅
├── DiffViewer.svelte          # Diff preview ✅ (bonus)
├── WorkspaceSelector.svelte   # Folder picker ✅ (bonus)
├── SessionHistory.svelte      # View completed sessions ✅ (bonus)
├── SessionTabs.svelte         # Multi-session tabs ✅ (bonus)
├── AgentConfig.svelte         # Agent selection ✅ (bonus)
└── index.ts                   # Exports ✅
```
