# Rust Language Features: Backend Architecture Patterns

This directory demonstrates Rust's trait system and modular architecture patterns for building extensible, pluggable backend systems.

## Beginner: Trait-Based Architecture

### The `Processor` Trait
```rust
#[async_trait]
pub trait Processor: Send + Sync {
    async fn process(&self, req: ProcessorRequest) -> ProcessorResponse;
}
```

**Key concepts:**
- **Traits define behavior**: Like interfaces in other languages, but more powerful
- **`Send + Sync`**: Ensures processors can be safely used across threads
- **Dynamic dispatch**: `dyn Processor` allows different processor types at runtime

### Trait Objects and Dynamic Dispatch
```rust
let processor: Arc<dyn Processor> = Arc::new(ChangeTextCaseProcessor::upper());
let response = processor.process(request).await;
```

**Why this works:**
- `dyn Processor` is a trait object - a pointer to any type implementing `Processor`
- Enables runtime polymorphism without generics
- `Arc<dyn Trait>` allows shared ownership of trait objects

## Intermediate: Factory Pattern and Type Erasure

### The Factory Pattern
```rust
pub struct LocalProcessorFactory;

impl LocalProcessorFactory {
    pub fn create_processor(config: &ProcessorConfig) -> Result<Arc<dyn Processor>, String> {
        match config.processor.as_ref()?.as_str() {
            "change_text_case_upper" => Ok(Arc::new(ChangeTextCaseProcessor::upper())),
            "reverse_text" => Ok(Arc::new(ReverseTextProcessor::new())),
            // ... more processors
            _ => Err(format!("Unknown processor: {}", config.processor.as_ref()?))
        }
    }
}
```

**Rust patterns demonstrated:**
- **Static dispatch to dynamic dispatch**: Concrete types become `Arc<dyn Processor>`
- **Error handling**: `Result<T, E>` for fallible operations
- **String matching**: Pattern matching on string slices
- **Option handling**: `as_ref()?` safely extracts from `Option<String>`

### Type Erasure Benefits
```rust
// All these different types become the same type after factory creation
let processors: Vec<Arc<dyn Processor>> = vec![
    LocalProcessorFactory::create_processor(&upper_config)?,
    LocalProcessorFactory::create_processor(&reverse_config)?,
    LocalProcessorFactory::create_processor(&counter_config)?,
];
```

**Why type erasure is powerful:**
- Uniform handling of heterogeneous processor types
- Runtime configuration determines which processors are created
- No need for complex generic type parameters

## Advanced: Modular Backend Architecture

### Backend Enumeration
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackendType {
    Local,
    Rpc,    // Future: Remote procedure call processors
    Wasm,   // Future: WebAssembly processors
}
```

**Design benefits:**
- **Extensible**: Easy to add new backend types
- **Type-safe**: Compiler ensures all backend types are handled
- **Serializable**: Can be configured via YAML/JSON

### Registry Pattern
```rust
pub fn build_registry(config: &Config) -> Result<HashMap<String, Arc<dyn Processor>>, String> {
    let mut registry = HashMap::new();
    
    for processor_config in &config.processors {
        let processor = match processor_config.backend {
            BackendType::Local => LocalProcessorFactory::create_processor(processor_config)?,
            BackendType::Rpc => todo!("RPC backend not yet implemented"),
            BackendType::Wasm => todo!("WASM backend not yet implemented"),
        };
        
        registry.insert(processor_config.id.clone(), processor);
    }
    
    Ok(registry)
}
```

**Advanced patterns:**
- **Centralized registration**: Single point for processor creation
- **Backend abstraction**: Same interface for different backend types
- **Configuration-driven**: Runtime behavior determined by config
- **Future-proofing**: Structure ready for additional backends

### Module Organization
```
src/backends/
├── mod.rs          # Public API and backend enumeration
├── local/          # Local (in-process) processors
│   ├── mod.rs
│   ├── factory.rs  # LocalProcessorFactory
│   └── processors/ # Individual processor implementations
└── stub.rs         # Stub implementation for testing
```

**Rust module system benefits:**
- **Encapsulation**: Each backend is self-contained
- **Selective exposure**: `pub use` controls what's publicly available
- **Hierarchical organization**: Clear separation of concerns

## Key Rust Concepts Demonstrated

### 1. **Trait Objects vs Generics**
```rust
// Generic approach (compile-time polymorphism)
fn process_with_generic<P: Processor>(processor: &P, input: ProcessorRequest) -> ProcessorResponse {
    processor.process(input)
}

// Trait object approach (runtime polymorphism)
fn process_with_trait_object(processor: &dyn Processor, input: ProcessorRequest) -> ProcessorResponse {
    processor.process(input)
}
```

**When to use each:**
- **Generics**: When types are known at compile time, better performance
- **Trait objects**: When types are determined at runtime, more flexible

### 2. **The `Arc<dyn Trait>` Pattern**
```rust
Arc<dyn Processor>  // Shared ownership + dynamic dispatch
```

**Why this combination:**
- `Arc`: Multiple owners can share the same processor instance
- `dyn`: Runtime polymorphism - different processor types behind same interface
- Thread-safe: Can be passed between async tasks

### 3. **Configuration-Driven Architecture**
```rust
#[derive(Deserialize)]
pub struct ProcessorConfig {
    pub id: String,
    pub backend: BackendType,
    pub processor: Option<String>,
    pub options: HashMap<String, serde_json::Value>,
}
```

**Benefits:**
- **Runtime flexibility**: Behavior determined by configuration files
- **No recompilation**: Change processors without rebuilding
- **Type safety**: Serde ensures configuration matches expected structure

## Design Principles Applied

1. **Open/Closed Principle**: Open for extension (new backends), closed for modification
2. **Dependency Inversion**: High-level code depends on abstractions (traits), not concrete types
3. **Single Responsibility**: Each backend handles one type of processor execution
4. **Interface Segregation**: Clean, minimal trait definitions

## Future Backend Implementations

### RPC Backend (Planned)
```rust
pub struct RpcProcessorFactory {
    client: RpcClient,
}

impl RpcProcessorFactory {
    pub fn create_processor(config: &ProcessorConfig) -> Result<Arc<dyn Processor>, String> {
        Ok(Arc::new(RpcProcessor::new(config.endpoint.clone()?)))
    }
}
```

### WASM Backend (Planned)
```rust
pub struct WasmProcessorFactory {
    runtime: WasmRuntime,
}

impl WasmProcessorFactory {
    pub fn create_processor(config: &ProcessorConfig) -> Result<Arc<dyn Processor>, String> {
        let module = self.runtime.load_module(&config.wasm_path.clone()?)?;
        Ok(Arc::new(WasmProcessor::new(module)))
    }
}
```

This backend architecture demonstrates how Rust's trait system enables building highly modular, extensible systems while maintaining type safety and performance.
