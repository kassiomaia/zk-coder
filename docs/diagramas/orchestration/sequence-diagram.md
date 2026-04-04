```mermaid
sequenceDiagram
    participant User
    participant Orchestrator
    participant NLPService as NLP Service
    participant Agent

    User->>Orchestrator: User request
    Orchestrator->>NLPService: Process input
    NLPService-->>Orchestrator: Processed data
    Orchestrator->>Agent: Execute task
    Agent-->>Orchestrator: Task result
    Orchestrator-->>User: Final response
```
