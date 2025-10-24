# The DAGwood

**The DAGwood** – A reference implementation for exploring DAG execution strategies and WebAssembly integration in workflow orchestration systems. This project demonstrates multiple execution approaches (reactive, level-by-level, work queue) and showcases secure WASM module/component integration with both classic core modules and modern Component Model components.

## 🎯 Purpose

This is a **learning and reference project** designed to:
- Explore and compare different DAG execution strategies
- Demonstrate WebAssembly integration patterns (classic modules and Component Model)
- Provide practical examples of workflow orchestration architecture
- Serve as a foundation for understanding trade-offs between execution approaches

**Note**: This is not production-ready software. It's an educational implementation for studying DAG execution and WASM integration patterns.

## ✨ Features

* **🚀 Multiple Execution Strategies**: Compare reactive, level-by-level, and work queue execution approaches
* **🔒 WASM Integration**: Support for both classic WASM modules (C-style) and modern Component Model components
* **🔧 Config-Driven**: Define entire workflows declaratively via YAML configuration
* **🎯 Unified Abstraction**: One consistent processor trait across all backends
* **📊 Rich Metadata**: Comprehensive execution metadata for analysis and debugging

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

| Strategy | Use Case | Architecture |
|----------|----------|--------------|
| **Reactive** | Low-latency, event-driven workflows | Event-driven notifications with async channels |
| **Level-by-Level** | Predictable, batch-oriented execution | Topological level computation with parallel batches |
| **Work Queue** | Complex DAGs with dynamic priorities | Dependency counting + priority queue |

### Processor Backends

* **Local**: In-process Rust processors for native execution
* **WASM**: Sandboxed execution supporting both classic modules (C-style) and Component Model components
  - Classic modules: Manual memory management with `allocate`/`deallocate` exports
  - Component Model: Automatic memory management via canonical ABI
* **Future**: RPC/gRPC support for distributed processing

### Key Components

* **DAG Execution Engine**: Pluggable strategies for different performance characteristics
* **Processor Registry**: Configuration-driven processor resolution and instantiation
* **Metadata System**: Rich execution context and performance metrics
* **Validation System**: Comprehensive DAG validation with cycle detection

## 📚 Documentation

* **[Walkthrough Guide](docs/walkthrough/)**: Comprehensive guide to the project architecture and implementation
* **[ADRs](docs/adrs/)**: Architectural Decision Records documenting key design choices
* **[Roadmap](ROADMAP.md)**: Project roadmap and implementation phases

## 🛣️ Project Status

See [ROADMAP.md](ROADMAP.md) for detailed implementation phases and current status.

## 📄 License

MIT - see [LICENSE](LICENSE) file for details.
