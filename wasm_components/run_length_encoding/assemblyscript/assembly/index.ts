/**
 * Run-Length Encoding WASM Component (AssemblyScript)
 * 
 * Implements RLE encoding for The DAGwood project.
 * Encodes consecutive characters: "aaabbc" -> "3a2b1c"
 */

/**
 * Performs run-length encoding on the input string.
 * 
 * @param input - The string to encode
 * @returns The RLE-encoded string
 * 
 * Examples:
 * - "aaabbc" -> "3a2b1c"
 * - "hello" -> "1h1e2l1o"
 * - "" -> ""
 * - "a" -> "1a"
 */
export function encodeRLE(input: string): string {
  if (input.length === 0) {
    return "";
  }

  let result = "";
  let i = 0;

  while (i < input.length) {
    const currentChar = input.charAt(i);
    let count = 1;

    // Count consecutive occurrences of the current character
    while (i + count < input.length && input.charAt(i + count) === currentChar) {
      count++;
    }

    // Append count and character to result
    result += count.toString() + currentChar;
    i += count;
  }

  return result;
}

/**
 * Main processing function that will be called by The DAGwood engine.
 * 
 * For now, this is a simple string-to-string transformation.
 * Future enhancement: Implement full Component Model WIT interface.
 * 
 * @param input - Input string to process
 * @returns RLE-encoded output string
 */
export function process(input: string): string {
  return encodeRLE(input);
}

/**
 * WASM memory allocation helper for string passing.
 * Required for proper memory management when calling from host.
 */
export function allocate(size: i32): i32 {
  return heap.alloc(size) as i32;
}

/**
 * WASM memory deallocation helper.
 * Required for proper memory management when calling from host.
 */
export function deallocate(ptr: i32, size: i32): void {
  heap.free(ptr as usize);
}
