# Run-Length Encoding - AssemblyScript Implementation

AssemblyScript WASM component that performs run-length encoding for The DAGwood project.

## Overview

This component encodes consecutive characters into count-character pairs:
- Input: `"aaabbc"`
- Output: `"3a2b1c"`

## Directory Structure

```
assemblyscript/
├── assembly/
│   └── index.ts          # Main RLE implementation
├── build/                # Compiled WASM output (gitignored)
├── test/
│   └── rle.test.js       # Test suite
├── package.json          # npm dependencies
├── asconfig.json         # AssemblyScript compiler config
├── Makefile              # Build automation
└── README.md             # This file
```

## Requirements

- **Node.js**: 22.12.0 (managed by asdf/mise)
- **npm**: Comes with Node.js
- **AssemblyScript**: Installed via npm (dev dependency)

## Building

```bash
# First time - installs dependencies and builds
make build

# Subsequent builds
make build
```

**Output**: `wasm_components/rle_assemblyscript.wasm`

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

### Current State
- Exports `encodeRLE()` and `process()` functions
- Uses AssemblyScript's native string handling
- Memory allocation helpers (`allocate`, `deallocate`) for WASM interop

### Future Enhancements
- Full Component Model WIT interface implementation
- Integration with `wasm-tools` for proper component wrapping
- Enhanced error handling with WIT error types
- Metadata support for DAGwood processor protocol

## AssemblyScript Specifics

AssemblyScript is a TypeScript-like language that compiles to WebAssembly:
- **Type Safety**: Strong typing enforced at compile time
- **Performance**: Near-native WASM performance
- **String Handling**: Built-in Unicode string support
- **Memory Management**: Automatic reference counting

## Integration with The DAGwood

Once built, configure in your DAGwood YAML:

```yaml
processors:
  - id: rle_encoder
    type: wasm
    module: wasm_components/rle_assemblyscript.wasm
    depends_on: [input_processor]
    options:
      intent: transform
```

## Development

### Modifying the RLE Logic

Edit `assembly/index.ts` and rebuild:

```bash
make build
```

### Adding Tests

Add test cases to `test/rle.test.js` and run:

```bash
make test
```

### Debugging

For debug build with source maps:

```bash
npm run build -- --target debug
```

This creates `build/debug.wasm` with source map support.

## Resources

- [AssemblyScript Documentation](https://www.assemblyscript.org/)
- [WebAssembly Component Model](https://component-model.bytecodealliance.org/)
- [The DAGwood WIT Interface](../../../wit/versions/v1.0.0/dagwood-processor.wit)
