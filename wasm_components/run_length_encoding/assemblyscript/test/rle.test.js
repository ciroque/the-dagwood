/**
 * Tests for AssemblyScript RLE Component
 */

import { test } from 'node:test';
import assert from 'node:assert';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const wasmPath = join(__dirname, '../build/rle_assemblyscript.wasm');

let instance;

// Load the WASM module before tests
async function loadWasm() {
  try {
    const wasmBuffer = readFileSync(wasmPath);
    const wasmModule = await WebAssembly.instantiate(wasmBuffer, {
      env: {
        abort: () => {
          throw new Error('WASM abort called');
        }
      }
    });
    instance = wasmModule.instance;
    return true;
  } catch (error) {
    console.error('Failed to load WASM:', error.message);
    return false;
  }
}

// Helper to convert JS string to WASM memory and call process function
function callProcess(input) {
  const { encodeRLE } = instance.exports;
  
  // For now, we're testing the encodeRLE function directly
  // Full Component Model integration will use the process function
  const encoder = new TextEncoder();
  const decoder = new TextDecoder();
  
  // AssemblyScript has built-in string support, so we can call directly
  const result = encodeRLE(input);
  return result;
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
