```mermaid
graph TD
    subgraph LLM
        A[llm.gateway]
        B[llm.models.code]
        C[llm.models.qa]
    end

    A --> B
    A --> C
```
