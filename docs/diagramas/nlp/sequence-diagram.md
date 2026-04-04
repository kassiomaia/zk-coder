```mermaid
sequenceDiagram
    participant User
    participant NLPService as NLP Service
    participant IntentRecognizer as Intent Recognizer
    participant EntityExtractor as Entity Extractor

    User->>NLPService: Process text
    NLPService->>IntentRecognizer: Recognize intent
    IntentRecognizer-->>NLPService: Intent
    NLPService->>EntityExtractor: Extract entities
    EntityExtractor-->>NLPService: Entities
    NLPService-->>User: Processed data
```
