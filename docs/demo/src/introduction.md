# The DAGwood Project Demo

Welcome to an interactive demonstration of **The DAGwood Project** - a modern workflow orchestration system built in Rust that showcases cutting-edge DAG execution strategies and WASM component integration.

## Demo Goals

This 10-15 minute demonstration highlights our four primary objectives:

### 🦀 **1. Learn Rust**
- **Ownership & Borrowing**: See how Rust's memory safety enables high-performance concurrent execution
- **Async/Await**: Discover how tokio powers our non-blocking DAG processors  
- **Trait System**: Explore how traits create pluggable execution strategies
- **Error Handling**: Experience Rust's `Result<T, E>` pattern for robust workflow orchestration

### 🔄 **2. Learn DAG Execution Strategies**
- **Work Queue + Dependency Counting**: Efficient topological execution with priority queues
- **Level-by-Level**: Batch processing with clear dependency boundaries
- **Reactive/Event-Driven**: Future implementation for real-time workflow orchestration
- **Hybrid Scheduling**: Advanced strategies combining multiple approaches

### 🧩 **3. Learn WASM Components**
- **Security Sandboxing**: True isolation using wasmtime runtime
- **Language Flexibility**: Support for Rust, C, AssemblyScript, and more
- **Performance**: Near-native execution with memory safety guarantees
- **Deterministic Execution**: Reproducible results across environments

### 🤖 **4. Use Generative AI Tools**
- **Accelerated Development**: How AI assistance enabled rapid prototyping
- **Learning Enhancement**: AI-guided exploration of complex Rust concepts
- **Code Quality**: AI-assisted refactoring and optimization
- **Documentation**: Comprehensive docs generated with AI collaboration

## What You'll See

### Progressive Complexity Journey
1. **Hello World** → Single processor basics
2. **Text Pipeline** → Linear data flow and chaining  
3. **Diamond Analysis** → Parallel execution and metadata collection
4. **WASM Integration** → Sandboxed processing with multiple languages
5. **Complex Workflow** → Real-world multi-backend orchestration

### Live Demonstrations
- **Interactive Execution**: Real DAG processing with live output
- **Configuration Examples**: YAML-driven workflow definitions
- **Performance Comparison**: Different execution strategies in action
- **Error Handling**: Graceful failure and recovery mechanisms

### Technical Deep-Dives
- **Architecture Decisions**: ADRs documenting key technical choices
- **Rust Best Practices**: Idiomatic patterns and performance optimizations
- **WASM Integration**: Cutting-edge sandboxing technology
- **Future Roadmap**: Planned enhancements and research directions

## Demo Format

This presentation uses **mdBook** - the same tool used by the official Rust documentation. You can:

- **Navigate** using the sidebar or arrow keys
- **Search** for specific topics using the search box
- **Copy code** examples with the copy button
- **Follow along** with the live terminal demonstrations

## Ready to Begin?

The demo follows a carefully crafted progression from simple concepts to advanced architectures. Each section builds on the previous one, culminating in a sophisticated workflow orchestration system that demonstrates the power of modern Rust development.

**Let's start with our first example: Hello World!**

---

> 💡 **Tip**: Keep the terminal window visible alongside this presentation to see the live execution results as we progress through each example.
