# Grain RLE Component Architecture

This document describes the architectural design and implementation details of the Grain-based run-length encoding WASM component for DAGwood.

## Design Philosophy

### Functional Programming First
The component is designed around Grain's functional programming paradigm:
- **Immutable Data**: All data structures are immutable by default
- **Pure Functions**: No side effects in core algorithm logic
- **Pattern Matching**: Leverages Grain's powerful pattern matching for clean control flow
- **Composability**: Functions are designed to be easily composed and tested

### WASM Integration Strategy
The architecture bridges functional programming with low-level WASM requirements:
- **Memory Management**: Explicit allocation/deallocation for WASM linear memory
- **Error Boundaries**: Clear separation between Grain errors and WASM interface errors
- **Data Serialization**: JSON-based communication with structured metadata

## Component Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    DAGwood Runtime                      │
│  ┌─────────────────────────────────────────────────┐    │
│  │              WasmProcessor                      │    │
│  │  ┌─────────────────────────────────────────┐    │    │
│  │  │           Grain WASM Module             │    │    │
│  │  │                                         │    │    │
│  │  │  ┌─────────────┐  ┌─────────────────┐  │    │    │
│  │  │  │ RLE Module  │  │  Main Module    │  │    │    │
│  │  │  │             │  │                 │  │    │    │
│  │  │  │ • encode()  │  │ • process()     │  │    │    │
│  │  │  │ • decode()  │  │ • allocate()    │  │    │    │
│  │  │  │ • auto()    │  │ • deallocate()  │  │    │    │
│  │  │  └─────────────┘  └─────────────────┘  │    │    │
│  │  └─────────────────────────────────────────┘    │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

## Module Design

### RLE Module (`src/rle.gr`)

#### Core Data Types
```grain
record RleSegment {
  count: Number,
  character: Char
}

enum RleResult<a, b> {
  Success(a),
  InvalidInput(b),
  TooLarge(b)
}
```

#### Algorithm Implementation
The RLE algorithm uses functional composition:

1. **String Explosion**: Convert string to character list
2. **Pattern Matching**: Group consecutive characters
3. **Counting**: Use list operations to count occurrences
4. **Reconstruction**: Build result using immutable operations

```grain
let rec encodeChars = (chars) => {
  match (chars) {
    [] => [],
    [char] => [{ count: 1, character: char }],
    [char, ...rest] => {
      let (sameChars, differentChars) = List.partition(
        (c) => Char.code(c) == Char.code(char), 
        rest
      )
      let count = 1 + List.length(sameChars)
      [{ count: count, character: char }, ...encodeChars(differentChars)]
    }
  }
}
```

#### Auto-Detection Logic
The component implements smart format detection:
- **Heuristic Analysis**: Check string patterns for encoding signatures
- **Fallback Strategy**: If decoding fails, treat as raw text
- **Error Recovery**: Graceful handling of ambiguous inputs

### Main Module (`src/main.gr`)

#### WASM Interface Implementation
The main module bridges Grain's high-level abstractions with WASM's low-level interface:

```grain
export let process = (inputPtr: Number, inputLen: Number, outputLenPtr: Number) => {
  // 1. Read from WASM linear memory
  // 2. Process using RLE module
  // 3. Serialize result to JSON
  // 4. Allocate output memory
  // 5. Write result and return pointer
}
```

#### Memory Management Strategy
- **Allocation Tracking**: Maintain list of allocated blocks
- **Error Safety**: Return null pointers on allocation failure
- **Cleanup**: Proper deallocation with size validation
- **Bounds Checking**: Prevent oversized allocations (1MB limit)

#### JSON Serialization
Structured output format for DAGwood integration:
```json
{
  "result": "processed_data",
  "metadata": {
    "operation": "encoded|decoded|error",
    "original_size": 123,
    "result_size": 456,
    "compression_ratio": 0.75,
    "algorithm": "run_length_encoding",
    "processor": "grain_rle"
  }
}
```

## Functional Programming Patterns

### Pattern Matching
Grain's pattern matching enables clean, readable algorithm implementation:
- **List Destructuring**: `[head, ...tail]` patterns for recursive processing
- **Option Handling**: Safe unwrapping of optional values
- **Error Propagation**: Match on Result types for error handling

### Immutable Data Structures
All data transformations create new structures:
- **No Mutation**: Original data is never modified
- **Memory Safety**: No dangling pointers or use-after-free
- **Concurrency Safe**: Immutable data can be safely shared

### Higher-Order Functions
Leverages Grain's functional library:
- **List.fold**: Accumulate results across collections
- **List.filter**: Select elements based on predicates
- **List.partition**: Split collections based on conditions

## Error Handling Architecture

### Three-Layer Error Strategy

1. **Grain Level**: Use Result types for algorithm errors
2. **WASM Level**: Return null pointers for interface errors
3. **JSON Level**: Structured error responses with metadata

### Error Categories
- **Processing Errors**: Invalid input, algorithm failures
- **Memory Errors**: Allocation failures, size limits
- **Interface Errors**: WASM boundary violations

### Recovery Mechanisms
- **Graceful Degradation**: Return error metadata instead of crashing
- **Null Pointer Safety**: Check allocations before use
- **Exception Boundaries**: Catch and convert Grain exceptions

## Performance Considerations

### Memory Efficiency
- **Functional Overhead**: Immutable structures have memory cost
- **Allocation Strategy**: Minimize WASM linear memory allocations
- **String Processing**: Efficient character list operations

### Computational Complexity
- **Encoding**: O(n) where n is input length
- **Decoding**: O(m) where m is number of segments
- **Memory**: O(n) for temporary data structures

### WASM Optimization
- **Grain Compiler**: Leverages Grain's WASM code generation
- **Size Optimization**: Functional code often compiles to compact WASM
- **Runtime Performance**: Grain's efficient runtime in WASM context

## Integration Points

### DAGwood WASM Backend
The component integrates with DAGwood's WASM infrastructure:
- **WasmProcessor**: Handles module loading and execution
- **Memory Management**: Coordinates with wasmtime runtime
- **Error Propagation**: Converts WASM errors to DAGwood errors

### Configuration Integration
```yaml
processors:
  - id: rle_grain
    backend: wasm
    module: wasm_modules/rle_grain.wasm
    options:
      intent: transform  # Modifies payload
```

### Metadata Flow
Rich metadata integration with DAGwood's metadata system:
- **Processing Stats**: Compression ratios, sizes, operation type
- **Performance Metrics**: Execution time, memory usage
- **Error Context**: Detailed error information for debugging

## Security Model

### Sandboxing
Complete isolation from host system:
- **No Imports**: Component imports no WASM interfaces
- **Memory Isolation**: Operates only within WASM linear memory
- **Deterministic**: No access to random numbers, time, or system state

### Input Validation
- **Size Limits**: Prevent oversized allocations
- **Format Validation**: Check input format before processing
- **Error Boundaries**: Contain errors within component

## Future Extensions

### Algorithm Enhancements
- **Adaptive RLE**: Choose encoding based on input characteristics
- **Multiple Formats**: Support different RLE variants
- **Binary Data**: Extend beyond text to arbitrary byte streams

### Performance Optimizations
- **Streaming**: Process data in chunks for large inputs
- **Parallel Processing**: Leverage WASM threads when available
- **Memory Pooling**: Reuse allocations for better performance

### Integration Features
- **Configuration**: Runtime configuration through metadata
- **Metrics**: Detailed performance and compression metrics
- **Debugging**: Enhanced error reporting and diagnostics
