# The DAGwood

**The DAGwood** – A high-performance, pluggable workflow orchestration engine that executes Directed Acyclic Graphs (DAGs) of processors. Features multiple execution strategies (reactive, level-by-level, work queue), WASM sandboxing, and a unified processor abstraction.

## ✨ Features

* **🚀 Multiple Execution Strategies**: Choose between reactive (fastest), level-by-level, or work queue execution
* **🔒 WASM Sandboxing**: Run processors in secure, isolated WASM environments
* **⚡ High Performance**: Reactive executor achieves ~300x faster execution than traditional work queues
* **🔧 Config-Driven**: Define entire workflows declaratively via YAML configuration
* **🎯 Unified Abstraction**: One consistent processor trait across all backends
* **📊 Rich Metadata**: Comprehensive execution metadata and performance metrics

## 🚀 Quick Start

### Prerequisites

* Rust (latest stable)
* Protobuf compiler (`protoc`)

### Build & Run

```bash
# Build the project
cargo build

# Run the interactive demo
cargo run -- --demo-mode

# Or run a specific strategy comparison
cargo run -- configs/strategy-workqueue-demo.yaml configs/strategy-reactive-demo.yaml configs/strategy-levelbylevel-demo.yaml "hello world"
```

### Configuration Example

```yaml
# Choose execution strategy: work_queue, level, or reactive
strategy: reactive
failure_strategy: fail_fast
executor_options:
  max_concurrency: 4

processors:
  - id: to_uppercase
    type: local
    processor: change_text_case_upper
    depends_on: []
    
  - id: reverse_text
    type: local
    processor: reverse_text
    depends_on: [to_uppercase]
    
  - id: add_brackets
    type: local
    processor: prefix_suffix_adder
    depends_on: [reverse_text]
    options:
      prefix: "["
      suffix: "]"
```

## 🏗️ Architecture

**The DAGwood** implements a pluggable execution architecture with three distinct strategies:

### Execution Strategies

| Strategy | Performance | Use Case | Architecture |
|----------|-------------|----------|--------------|
| **Reactive** | ~300x faster | Low-latency, real-time | Event-driven notifications |
| **Level-by-Level** | ~77x faster | Predictable execution | Topological level batching |
| **Work Queue** | Baseline | Complex DAGs, production | Dependency counting + priority queue |

### Processor Backends

* **Local**: In-process Rust processors with high performance
* **WASM**: Sandboxed execution with wasmtime for security isolation
* **Future**: RPC/gRPC support for distributed processing

### Key Components

* **DAG Execution Engine**: Pluggable strategies for different performance characteristics
* **Processor Registry**: Configuration-driven processor resolution and instantiation
* **Metadata System**: Rich execution context and performance metrics
* **Validation System**: Comprehensive DAG validation with cycle detection

## 📈 Performance Results

**Test Pipeline**: `"hello world"` → uppercase → reverse → add brackets → `"[DLROW OLLEH]"`

| Strategy | Execution Time | Relative Performance |
|----------|----------------|---------------------|
| **Reactive** | 224μs | **~300x faster** ⚡ |
| **Level-by-Level** | 889μs | ~77x faster |
| **WorkQueue** | 68.6ms | Baseline |

*Results demonstrate that simpler architectures can dramatically outperform complex coordination systems.*

## 🛣️ Roadmap

* [x] ✅ Multiple DAG execution strategies (reactive, level-by-level, work queue)
* [x] ✅ WASM sandboxing with wasmtime integration
* [x] ✅ Comprehensive validation and error handling
* [x] ✅ Rich metadata collection and performance metrics
* [ ] 🔄 RPC/gRPC backend for distributed processing
* [ ] 🔄 Observability hooks (OpenTelemetry integration)
* [ ] 🔄 Dynamic strategy selection and A/B testing
* [ ] 🔄 Machine learning-based runtime optimization

## 📄 License

MIT - see [LICENSE](LICENSE) file for details.
