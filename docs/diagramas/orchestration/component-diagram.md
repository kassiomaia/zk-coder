```mermaid
graph TD
    subgraph Orchestrator
        A[Request Dispatcher]
        B[Workflow Manager]
        C[Result Aggregator]
    end

    A --> B
    B --> C
```
