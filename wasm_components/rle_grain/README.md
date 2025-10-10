# Grain RLE WASM Component

A run-length encoding (RLE) processor implemented in [Grain](https://grain-lang.org/) for the DAGwood workflow orchestration system. This component demonstrates Grain's functional programming strengths in a real-world WASM application.

## Overview

This component implements bidirectional run-length encoding with auto-detection, showcasing:

- **Functional Programming**: Pattern matching, immutable data structures, higher-order functions
- **WASM Integration**: Full DAGwood WIT interface compliance with memory management
- **Smart Processing**: Auto-detects whether input is encoded or raw text
- **Rich Metadata**: Provides compression statistics and processing information

## Features

### Run-Length Encoding
- **Encode**: Converts text like `"aaabbc"` to `"3a2b1c"`
- **Decode**: Converts encoded format back to original text
- **Auto-detect**: Automatically determines if input needs encoding or decoding
- **Compression Stats**: Reports original size, compressed size, and compression ratio

### Functional Programming Showcase
- **Pattern Matching**: Clean handling of different input cases
- **Immutable Data**: All transformations create new data structures
- **Higher-Order Functions**: Uses `map`, `fold`, `filter` for data processing
- **Algebraic Data Types**: Type-safe error handling with `Result` types

### WASM Interface Compliance
- **Memory Management**: Proper `allocate`/`deallocate` implementation
- **Error Handling**: Graceful failure with meaningful error messages
- **JSON Output**: Structured results with metadata for DAGwood integration

## Usage

### Building

```bash
# Install Grain (requires Node.js)
npm install -g @grain-lang/cli

# Build the WASM component
./build.sh
```

### DAGwood Configuration

```yaml
processors:
  - id: rle_processor
    backend: wasm
    module: wasm_modules/rle_grain.wasm
    depends_on: [input_processor]
    options:
      intent: transform
```

### Input/Output Examples

#### Encoding Example
**Input**: `"hello world"`
**Output**:
```json
{
  "result": "1h1e2l1o1 1w1o1r1l1d",
  "metadata": {
    "operation": "encoded",
    "original_size": 11,
    "result_size": 21,
    "compression_ratio": 1.91,
    "algorithm": "run_length_encoding",
    "processor": "grain_rle"
  }
}
```

#### Decoding Example
**Input**: `"3a2b1c"`
**Output**:
```json
{
  "result": "aaabbc",
  "metadata": {
    "operation": "decoded",
    "original_size": 6,
    "result_size": 6,
    "compression_ratio": 1.0,
    "algorithm": "run_length_encoding",
    "processor": "grain_rle"
  }
}
```

## Architecture

### Module Structure
```
src/
├── rle.gr          # Core RLE algorithm implementation
└── main.gr         # WASM interface and DAGwood integration
```

### Key Components

#### RLE Module (`src/rle.gr`)
- **`encode()`**: Converts string to RLE segments using pattern matching
- **`decode()`**: Reconstructs original string from RLE segments
- **`processAuto()`**: Auto-detects input format and processes accordingly
- **`compressionStats()`**: Calculates compression metrics

#### Main Module (`src/main.gr`)
- **`process()`**: DAGwood WIT interface implementation
- **`allocate()`**: WASM linear memory allocation
- **`deallocate()`**: Memory cleanup and management
- **JSON serialization**: Structured output with metadata

### Functional Programming Patterns

#### Pattern Matching
```grain
match (chars) {
  [] => [],
  [char] => [{ count: 1, character: char }],
  [char, ...rest] => {
    // Process consecutive characters
  }
}
```

#### Immutable Data Structures
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

#### Higher-Order Functions
```grain
List.fold((acc, segment) => acc ++ segmentToString(segment), "", segments)
```

## Performance Characteristics

- **Memory Efficient**: Functional approach with minimal allocations
- **Deterministic**: Same input always produces same output
- **Sandboxed**: Complete isolation from host system
- **Fast Compilation**: Grain's efficient WASM code generation

## Error Handling

The component provides comprehensive error handling:

- **Invalid Input**: Malformed data or encoding errors
- **Memory Errors**: Allocation failures or size limits
- **Processing Errors**: Algorithm failures or unexpected conditions

All errors are returned as structured JSON with error details and metadata.

## Integration with DAGwood

This component integrates seamlessly with DAGwood's WASM backend:

1. **Security**: Complete sandboxing with no host system access
2. **Memory Management**: Proper linear memory handling
3. **Metadata**: Rich processing information for pipeline analysis
4. **Performance**: Efficient WASM execution with wasmtime runtime

## Development

### Prerequisites
- [Grain](https://grain-lang.org/) compiler
- Node.js (for Grain installation)
- Basic understanding of functional programming concepts

### Testing
```bash
# Run Grain tests (if test framework is set up)
grain test

# Manual testing with DAGwood integration
# Use the DAGwood test suite with this component
```

### Extending
The modular design makes it easy to:
- Add new compression algorithms
- Implement different encoding formats
- Extend metadata collection
- Add configuration options

## Learning Resources

- [Grain Language Documentation](https://grain-lang.org/docs/)
- [Functional Programming Concepts](https://grain-lang.org/docs/guide/functions)
- [Pattern Matching in Grain](https://grain-lang.org/docs/guide/pattern_matching)
- [DAGwood WASM Architecture](../../docs/walkthrough/src/wasm-architecture.md)
