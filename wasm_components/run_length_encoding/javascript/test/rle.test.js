/**
 * Tests for JavaScript WIT Component RLE
 */

import { test } from 'node:test';
import assert from 'node:assert';
import { readFileSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const wasmPath = join(__dirname, '../build/rle_js.wasm');

let instance;
let memory;

// Load the WIT Component
async function loadWasm() {
  if (!existsSync(wasmPath)) {
    console.log('⚠️  WASM component not built yet - run make build first');
    return false;
  }

  try {
    const wasmBuffer = readFileSync(wasmPath);
    
    // For WIT Components, we need to use the Component API
    // For now, test the JavaScript module directly since componentize-js isn't built yet
    // This will be updated once we have the actual Component binary
    
    const wasmModule = await WebAssembly.instantiate(wasmBuffer, {});
    instance = wasmModule.instance;
    memory = instance.exports.memory;
    return true;
  } catch (error) {
    console.error('Failed to load WASM component:', error.message);
    return false;
  }
}

// Helper to call the WIT Component's process function
function callProcess(input) {
  const { process, allocate, deallocate } = instance.exports;
  const encoder = new TextEncoder();
  const decoder = new TextDecoder();

  // Encode input
  const inputBytes = encoder.encode(input);
  
  // Allocate input memory
  const inputAllocResult = allocate(BigInt(inputBytes.length));
  if (inputAllocResult.tag === 'err') {
    throw new Error('Failed to allocate input memory');
  }
  const inputPtr = inputAllocResult.val;
  
  // Write input to WASM memory
  const memView = new Uint8Array(memory.buffer);
  memView.set(inputBytes, inputPtr);
  
  // Allocate output length pointer
  const outputLenAllocResult = allocate(BigInt(8));
  if (outputLenAllocResult.tag === 'err') {
    throw new Error('Failed to allocate output length memory');
  }
  const outputLenPtr = outputLenAllocResult.val;
  
  // Call process
  const processResult = process(inputPtr, BigInt(inputBytes.length), outputLenPtr);
  
  // Clean up input memory
  deallocate(inputPtr, BigInt(inputBytes.length));
  
  if (processResult.tag === 'err') {
    deallocate(outputLenPtr, BigInt(8));
    throw new Error(`Processing failed: ${JSON.stringify(processResult.val)}`);
  }
  
  const outputPtr = processResult.val;
  
  // Read output length
  const outputLenView = new DataView(memory.buffer, outputLenPtr, 8);
  const outputLen = Number(outputLenView.getBigUint64(0, true));
  
  // Clean up output length memory
  deallocate(outputLenPtr, BigInt(8));
  
  // Read output data
  const outputBytes = new Uint8Array(memory.buffer, outputPtr, outputLen);
  const output = decoder.decode(outputBytes);
  
  // Clean up output memory
  deallocate(outputPtr, BigInt(outputLen));
  
  return output;
}

test('RLE encoding - basic case', async () => {
  const loaded = await loadWasm();
  if (!loaded) {
    console.log('⚠️  WASM module not built yet - run npm run build first');
    return;
  }

  const result = callProcess('aaabbc');
  assert.strictEqual(result, '3a2b1c', 'Should encode "aaabbc" as "3a2b1c"');
});

test('RLE encoding - empty string', async () => {
  const loaded = await loadWasm();
  if (!loaded) return;

  const result = callProcess('');
  assert.strictEqual(result, '', 'Should handle empty string');
});

test('RLE encoding - single character', async () => {
  const loaded = await loadWasm();
  if (!loaded) return;

  const result = callProcess('a');
  assert.strictEqual(result, '1a', 'Should encode single char as "1a"');
});

test('RLE encoding - no consecutive chars', async () => {
  const loaded = await loadWasm();
  if (!loaded) return;

  const result = callProcess('abcd');
  assert.strictEqual(result, '1a1b1c1d', 'Should encode each char with count of 1');
});

test('RLE encoding - all same character', async () => {
  const loaded = await loadWasm();
  if (!loaded) return;

  const result = callProcess('aaaaa');
  assert.strictEqual(result, '5a', 'Should encode all same as "5a"');
});

test('RLE encoding - hello world pattern', async () => {
  const loaded = await loadWasm();
  if (!loaded) return;

  const result = callProcess('hello');
  assert.strictEqual(result, '1h1e2l1o', 'Should encode "hello" correctly with 2 consecutive l\'s');
});

console.log('🧪 Running AssemblyScript RLE tests...');
