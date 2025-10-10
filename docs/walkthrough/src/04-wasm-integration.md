# WASM Integration: Sandboxed Processing

## What You'll See

This demonstration introduces WebAssembly (WASM) processors, showcasing cutting-edge sandboxing technology and multi-language support. You'll see how DAGwood integrates WASM modules for secure, isolated execution alongside native Rust processors.

**Key Learning Points:**
- WASM module loading and execution with wasmtime
- Memory management across WASM boundaries
- Security sandboxing and isolation patterns
- Multi-backend processor architecture

## The Demo

### Command Line

```bash
cargo run --release -- docs/walkthrough/configs/04-wasm-integration.yaml "hello world"
```

### Configuration

```yaml
# Demo 4: WASM Integration - Sandboxed Processing
# This demonstrates WASM processor integration with security sandboxing

strategy: work_queue
failure_strategy: fail_fast

executor_options:
  max_concurrency: 2

processors:
  # Local processor prepares input
  - id: prepare_input
    type: local
    processor: change_text_case_lower
    depends_on: []
    options: {}

  # WASM processor provides sandboxed execution
  - id: wasm_hello_world
    type: wasm
    module: wasm_components/hello.wasm
    depends_on: [prepare_input]
    options:
      intent: transform

  # Local processor adds final formatting
  - id: final_format
    type: local
    processor: prefix_suffix_adder
    depends_on: [wasm_hello_world]
    options:
      prefix: "🦀 Rust + WASM: "
      suffix: " ✨"
```

**Configuration Elements:**
- **Strategy**: `work_queue` (dependency counting algorithm)
- **Failure Strategy**: `fail_fast` (stop on first error)
- **Concurrency**: Set to 2 (mixed backend execution)
- **Multi-Backend Pipeline**: Local → WASM → Local

### Expected Output

When you run this demo, you'll see:

```
🚀 DAGwood Execution Strategy Demo
═══════════════════════════════════
Input: "hello world"
Config files: ["docs/walkthrough/configs/04-wasm-integration.yaml"]

📋 Configuration: docs/walkthrough/configs/04-wasm-integration.yaml
🔧 Strategy: WorkQueue
⚙️  Max Concurrency: 2
🛡️  Failure Strategy: FailFast

📊 Execution Results:
⏱️  Execution Time: ~5ms
🔢 Processors Executed: 3

🔄 Processor Chain:
  1. prepare_input → "hello world"
  2. wasm_hello_world → "hello world-wasm" (WASM)
  3. final_format → "🦀 Rust + WASM: hello world-wasm ✨"

🎯 Final Transformation:
   Input:  "hello world"
   Output: "🦀 Rust + WASM: hello world-wasm ✨"
```

## What You Just Saw

This demo demonstrated:

**Multi-Backend Integration:**
- Seamless integration between Local and WASM backends
- Mixed execution pipeline: Local → WASM → Local
- Consistent processing-node interface across different backends

**WASM Security and Isolation:**
- Complete sandboxing with wasmtime runtime
- Memory isolation between host and WASM module
- Controlled capabilities with no host system access

**Rust System Programming:**
- C-style FFI for WASM compatibility
- Manual memory management across boundaries
- Resource cleanup and ownership transfer patterns
- Error propagation with `?` operator

**Cross-Language Potential:**
- WASM modules can be written in multiple languages
- Consistent interface regardless of implementation language
- Future-proof architecture for polyglot processing

### Performance Characteristics

WASM execution has different performance characteristics:

- **Startup cost**: Module loading and instantiation (~1-2ms)
- **Execution speed**: Near-native performance for compute-intensive tasks
- **Memory overhead**: Separate linear memory space
- **Security overhead**: Sandboxing adds minimal runtime cost

## Security Analysis

### Threat Model

WASM processors provide defense against:

- **Malicious code execution**: Complete sandboxing prevents host compromise
- **Resource exhaustion**: Memory and CPU limits can be enforced
- **Data exfiltration**: No network or file system access
- **Side-channel attacks**: Isolated execution environment

### Trust Boundaries

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Local Proc    │    │   WASM Proc     │    │   Local Proc    │
│   (Trusted)     │───▶│  (Sandboxed)    │───▶│   (Trusted)     │
│                 │    │                 │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
        │                        │                        │
        ▼                        ▼                        ▼
   Host Memory              WASM Memory               Host Memory
```

## Try It Yourself

### Building the WASM Module

```bash
# Install WASM target
rustup target add wasm32-unknown-unknown

# Build the module
cd wasm_components/wasm_appender
cargo build --target wasm32-unknown-unknown --release
```

### Experimenting with WASM

1. **Modify the WASM logic**: Change the `-wasm` suffix to something else
2. **Add computation**: Implement a more complex algorithm in WASM
3. **Test isolation**: Try to access host resources (it should fail!)

## What's Next?

In the final demo, the exploration moves to a **complex multi-backend workflow** that combines everything learned:
- Multiple execution strategies
- Mixed local and WASM processors
- Advanced error handling
- Production-ready patterns

---

> 🔒 **Security Insight**: WASM represents the future of secure code execution. By combining Rust's memory safety with WASM's sandboxing, both performance and security are achieved - essential for processing untrusted code in production environments!
