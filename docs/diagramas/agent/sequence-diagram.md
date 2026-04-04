```mermaid
sequenceDiagram
    participant User
    participant Agent
    participant TaskManager as Task Manager
    participant ToolExecutor as Tool Executor

    User->>Agent: Execute task
    Agent->>TaskManager: Create task
    TaskManager->>ToolExecutor: Execute tool
    ToolExecutor-->>TaskManager: Tool result
    TaskManager-->>Agent: Task result
    Agent-->>User: Task feedback
```
