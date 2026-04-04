```mermaid
graph TD
    A[Agent] --> B{LLM Service};
    B --> C[Generate Code];
    B --> D[Answer Questions];
    C --> E[Code Generation Model];
    D --> F[Question Answering Model];
```
