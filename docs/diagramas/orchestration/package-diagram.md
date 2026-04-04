```mermaid
graph TD
    subgraph Orchestration
        A[orchestration.dispatcher]
        B[orchestration.workflow]
        C[orchestration.aggregator]
    end

    A --> B
    B --> C
```
