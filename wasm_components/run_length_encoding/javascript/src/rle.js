/**
 * Run-Length Encoding WIT Component (JavaScript)
 * 
 * Implements the dagwood:component/processing-node interface
 * for run-length encoding of text data.
 * 
 * Encodes consecutive characters: "aaabbc" -> "3a2b1c"
 */

/**
 * Performs run-length encoding on the input string.
 * 
 * @param {string} input - The string to encode
 * @returns {string} The RLE-encoded string
 * 
 * Examples:
 * - "aaabbc" -> "3a2b1c"
 * - "hello" -> "1h1e2l1o"
 * - "" -> ""
 * - "a" -> "1a"
 */
function encodeRLE(input) {
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

// Memory management for componentize-js
let memory;
let heap = new Uint8Array(1024 * 64); // 64KB initial heap
let heapPointer = 0;

/**
 * Allocate memory in the WASM linear memory space
 * 
 * @param {bigint} size - Size to allocate in bytes
 * @returns {{tag: string, val: number} | {tag: string, val: {tag: string, val: bigint}}} 
 *          Success with pointer or error
 */
function allocate(size) {
  const sizeNum = Number(size);
  
  if (sizeNum === 0 || sizeNum > 1024 * 1024) {
    return { tag: 'err', val: { tag: 'invalid-size', val: size } };
  }

  // Ensure we have enough space
  if (heapPointer + sizeNum > heap.length) {
    // Grow heap
    const newSize = Math.max(heap.length * 2, heapPointer + sizeNum);
    const newHeap = new Uint8Array(newSize);
    newHeap.set(heap);
    heap = newHeap;
  }

  const ptr = heapPointer;
  heapPointer += sizeNum;

  return { tag: 'ok', val: ptr };
}

/**
 * Deallocate memory (currently a no-op, relies on GC)
 * 
 * @param {number} ptr - Pointer to deallocate
 * @param {bigint} size - Size of allocation
 */
function deallocate(ptr, size) {
  // Simple bump allocator - deallocation is a no-op
  // In production, you'd implement a proper allocator
}

/**
 * Process input data with RLE encoding
 * 
 * @param {number} inputPtr - Pointer to input data
 * @param {bigint} inputLen - Length of input data
 * @param {number} outputLenPtr - Pointer to write output length
 * @returns {{tag: string, val: number} | {tag: string, val: {tag: string, val: string}}}
 *          Success with output pointer or error
 */
function process(inputPtr, inputLen, outputLenPtr) {
  try {
    const inputLenNum = Number(inputLen);

    // Read input data from heap
    const inputBytes = heap.slice(inputPtr, inputPtr + inputLenNum);
    const inputText = new TextDecoder().decode(inputBytes);

    // Perform RLE encoding
    const outputText = encodeRLE(inputText);

    // Encode output as UTF-8
    const outputBytes = new TextEncoder().encode(outputText);
    const outputLen = outputBytes.length;

    // Allocate memory for output
    const allocResult = allocate(BigInt(outputLen));
    if (allocResult.tag === 'err') {
      return { 
        tag: 'err', 
        val: { 
          tag: 'processing-failed', 
          val: 'Failed to allocate output memory' 
        } 
      };
    }

    const outputPtr = allocResult.val;

    // Write output data to heap
    heap.set(outputBytes, outputPtr);

    // Write output length to the provided pointer
    const outputLenBytes = new Uint8Array(8);
    new DataView(outputLenBytes.buffer).setBigUint64(0, BigInt(outputLen), true);
    heap.set(outputLenBytes, outputLenPtr);

    return { tag: 'ok', val: outputPtr };

  } catch (error) {
    return { 
      tag: 'err', 
      val: { 
        tag: 'processing-failed', 
        val: error.message || 'Unknown error' 
      } 
    };
  }
}

// Export the processing-node interface
// The WIT interface name "processing-node" becomes "processingNode" in JavaScript
export const processingNode = {
  process,
  allocate,
  deallocate
};
