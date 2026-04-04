```mermaid
graph TD
    subgraph Agent
        A[Agent Core]
        B[Task Manager]
        C[State Manager]
        D[Tool Executor]
    end

    A --> B
    A --> C
    A --> D
```
