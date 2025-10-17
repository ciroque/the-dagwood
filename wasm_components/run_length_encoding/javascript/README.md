# Run-Length Encoding - JavaScript WIT Component

JavaScript WIT Component that performs run-length encoding for The DAGwood project using the Component Model.

## Overview

This component encodes consecutive characters into count-character pairs:
- Input: `"aaabbc"`
- Output: `"3a2b1c"`

**Key Features:**
- ✅ Implements WIT Component Model (not core WASM)
- ✅ Uses `componentize-js` for proper Component generation
- ✅ Implements `dagwood:component/processing-node` interface
- ✅ Works with `WitNodeExecutor` in DAGwood

## Directory Structure

```
assemblyscript/  (legacy name - contains JavaScript now)
├── src/
│   └── rle.js            # Main RLE implementation
├── build/                # Compiled WASM Component (gitignored)
├── test/
│   └── rle.test.js       # Test suite
├── package.json          # npm dependencies (jco, componentize-js)
├── Makefile              # Build automation
└── README.md             # This file
```

## Requirements

- **Node.js**: 22.12.0 (managed by asdf/mise)
- **npm**: Comes with Node.js
- **jco**: Installed via npm (dev dependency)
- **componentize-js**: Installed via npm (dev dependency)

## Building

```bash
# First time - installs dependencies and builds
make build

# Subsequent builds
make build
```

**Output**: `wasm_components/rle_js.wasm` (a proper WIT Component)

## Testing

```bash
make test
```

Runs the test suite with Node.js test runner. Tests cover:
- Basic RLE encoding (`"aaabbc"` → `"3a2b1c"`)
- Empty strings
- Single characters
- No consecutive characters
- All same character
- Common patterns (`"hello"` → `"1h1e2l1o"`)

## Cleaning

```bash
make clean
```

Removes:
- Build artifacts (`build/`)
- Node modules (`node_modules/`)
- Package lock file
- Output WASM in `wasm_components/`

## Implementation Notes

### WIT Interface Implementation
This component implements the `dagwood:component/processing-node` interface defined in `wit/versions/v1.0.0/dagwood-processor.wit`:

```wit
interface processing-node {
    process: func(input-ptr: u32, input-len: u64, output-len-ptr: u32) 
        -> result<u32, processing-error>;
    allocate: func(size: u64) -> result<u32, allocation-error>;
    deallocate: func(ptr: u32, size: u64);
}
```

### JavaScript Implementation
- **Core Logic**: Pure JavaScript RLE encoding (`encodeRLE` function)
- **Memory Management**: Simple bump allocator for WASM linear memory
- **WIT Bindings**: `componentize-js` generates proper Component Model bindings
- **Error Handling**: Returns WIT `result` types with proper error variants
- **Type Safety**: Uses proper protobuf structure for DAGwood integration

### Component Model Benefits
- **Type-Safe**: WIT interface ensures type safety across language boundaries
- **Composable**: Can be imported/exported by other components
- **Language-Agnostic**: Works with any WIT-compatible host
- **Future-Proof**: Part of the WebAssembly Component Model standard

## Integration with The DAGwood

Once built, configure in your DAGwood YAML:

```yaml
processors:
  - id: rle_encoder
    type: wasm
    module: wasm_components/rle_js.wasm
    depends_on: [input_processor]
    options:
      intent: transform
```

**Note**: This will work once `WitNodeExecutor` is implemented in DAGwood.

## Development

### Modifying the RLE Logic

Edit `src/rle.js` and rebuild:

```bash
make build
```

The JavaScript code is simple and straightforward - no complex build toolchain needed!

### Adding Tests

Add test cases to `test/rle.test.js` and run:

```bash
make test
```

### Understanding componentize-js

The build process uses `jco componentize` which:
1. Takes your JavaScript code
2. Reads the WIT interface definition
3. Generates proper Component Model bindings
4. Creates a `.wasm` file that's a true WIT Component (not just core WASM)

This is what makes it work with `WitNodeExecutor` - it's a proper Component Model component!

## Why JavaScript Instead of AssemblyScript?

**Original Plan**: AssemblyScript → Core WASM module
- ❌ Required custom runtime imports (`env.abort`, etc.)
- ❌ Would need a special "CoreNodeExecutor" just for this
- ❌ Not compatible with Component Model
- ❌ Dying approach, not the future

**Current Implementation**: JavaScript → WIT Component
- ✅ First-class Component Model support via `componentize-js`
- ✅ Works with `WitNodeExecutor` (the proper way)
- ✅ Industry standard (Bytecode Alliance tooling)
- ✅ Future-proof and composable
- ✅ Simple JavaScript - no complex build chain

## Resources

- [componentize-js Documentation](https://github.com/bytecodealliance/componentize-js)
- [jco Tools](https://github.com/bytecodealliance/jco)
- [WebAssembly Component Model](https://component-model.bytecodealliance.org/)
- [The DAGwood WIT Interface](../../../wit/versions/v1.0.0/dagwood-processor.wit)
- [Bytecode Alliance](https://bytecodealliance.org/)
