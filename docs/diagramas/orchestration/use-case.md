```mermaid
graph TD
    A[User] --> B{Orchestrator};
    B --> C[NLP Service];
    B --> D[Agent];
    C --> E[Process User Input];
    D --> F[Execute Task];
    E --> B;
    F --> B;
```
