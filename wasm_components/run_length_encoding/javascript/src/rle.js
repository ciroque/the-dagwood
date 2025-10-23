/**
 * Run-Length Encoding WIT Component (JavaScript)
 * 
 * Implements the dagwood:component/processing-node interface
 * for run-length encoding of text data.
 * 
 * Encodes consecutive characters: "aaabbc" -> "3a2b1c"
 * 
 * Uses the new list-based WIT interface:
 *   process: func(input: list<u8>) -> result<list<u8>, processing-error>
 * 
 * componentize-js handles all memory management automatically!
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

/**
 * Process input data with RLE encoding
 * 
 * @param {Uint8Array} input - Input bytes (componentize-js provides this)
 * @returns {{tag: 'ok', val: Uint8Array} | {tag: 'err', val: {tag: string, val: string}}}
 *          WIT result type
 */
function process(input) {
  try {
    console.log('[RLE] Input received:', input);
    console.log('[RLE] Input length:', input ? input.length : 'null');
    
    // Decode input bytes to string
    const inputText = new TextDecoder().decode(input);
    console.log('[RLE] Decoded input text:', inputText);

    // Perform RLE encoding
    const outputText = encodeRLE(inputText);
    console.log('[RLE] Encoded output text:', outputText);

    // Encode output as UTF-8 bytes - convert to Array for WIT list<u8>
    const outputBytes = Array.from(new TextEncoder().encode(outputText));
    console.log('[RLE] Output bytes length:', outputBytes.length);

    // Return success with output bytes
    return { tag: 'ok', val: outputBytes };

  } catch (error) {
    console.error('[RLE] Error:', error);
    // Return error with processing-failed variant
    return { 
      tag: 'err', 
      val: { 
        tag: 'processing-failed', 
        val: error.message || 'Unknown error during RLE encoding' 
      } 
    };
  }
}

// Export the processing-node interface
// WIT "processing-node" interface becomes "processingNode" in JavaScript (camelCase)
export const processingNode = {
  process
};
