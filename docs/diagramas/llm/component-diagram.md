```mermaid
graph TD
    subgraph LLM Service
        A[LLM Gateway]
        B[Code Generation Model]
        C[Question Answering Model]
    end

    A --> B
    A --> C
```
