```mermaid
graph TD
    subgraph Agent
        A[agent.core]
        B[agent.tasks]
        C[agent.state]
        D[agent.tools]
    end

    A --> B
    A --> C
    A --> D
```
