# ADR 7: Persistence of DAG Definitions (Deferred)

## Context

Pipelines are currently assembled from configuration files loaded at runtime. For the proof-of-concept, it is sufficient to support static configuration. However, in production environments, teams may require persistence of DAG definitions for:

* Storing and retrieving pipelines from databases or remote stores.
* Versioning and auditing changes to pipeline definitions.
* Sharing and distributing pipeline definitions across services or teams.
* Supporting hot reload and dynamic updates without restarts.

Several persistence approaches exist:

* **File-based (YAML/TOML/JSON)** persisted in VCS.
* **Database-backed (SQL/NoSQL)** to enable queries, versioning, and access control.
* **Service API** that manages pipelines as first-class entities, possibly with a UI.

## Decision

Persistence of DAG definitions will be **deferred**. The proof-of-concept will only support loading configurations from local files. Future iterations will revisit persistence once the execution engine and plugin model are validated.

## Consequences

* **Short-term:** Simple and fast to implement for POC.
* **Flexibility:** We can experiment with DAG structures without committing to a persistence model.
* **Limitations:** No built-in versioning, sharing, or remote management initially.
* **Future ADRs:** When persistence becomes a priority, a new ADR will evaluate options such as file-based VCS, database-backed storage, or service-oriented persistence APIs.
