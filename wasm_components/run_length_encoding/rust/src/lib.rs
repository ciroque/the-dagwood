// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! Run Length Encoding WASM Component - Component Model Interface
//!
//! This component implements standard Run Length Encoding (RLE) compression
//! for ASCII text input using the WebAssembly Component Model interface.
//! It exports:
//! - process: func(input: list<u8>) -> result<list<u8>, processing-error>
//!
//! Memory management is handled automatically by wit-bindgen!

mod bindings;

use bindings::exports::dagwood::component::processing_node::{Guest, ProcessingError};

// Maximum input size to prevent memory exhaustion (adjust as needed)
const MAX_INPUT_SIZE: u64 = 1024 * 1024; // 1MB

struct Component;

impl Guest for Component {
    fn process(input: Vec<u8>) -> Result<Vec<u8>, ProcessingError> {
        // Check input size
        if input.len() as u64 > MAX_INPUT_SIZE {
            return Err(ProcessingError::InputTooLarge(input.len() as u64));
        }

        // Convert input bytes to string (ASCII-only for simplicity)
        let input_str = match std::str::from_utf8(&input) {
            Ok(s) => s,
            Err(e) => {
                return Err(ProcessingError::InvalidInput(
                    format!("Invalid UTF-8: {}", e)
                ));
            }
        };

        // Validate ASCII-only input
        if !input_str.is_ascii() {
            return Err(ProcessingError::InvalidInput(
                "Non-ASCII characters are not supported".to_string()
            ));
        }

        // Perform RLE encoding
        let encoded = run_length_encode(input_str);

        // Return as bytes
        Ok(encoded.into_bytes())
    }
}

/// Performs Run Length Encoding on the input ASCII string.
///
/// Converts sequences of repeated characters into count+character pairs.
/// For example: "aaabbc" → "3a2b1c"
fn run_length_encode(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    let chars: Vec<char> = input.chars().collect();

    let mut i = 0;
    while i < chars.len() {
        let current_char = chars[i];
        let mut count = 1;

        // Count consecutive occurrences of the current character
        while i + count < chars.len() && chars[i + count] == current_char {
            count += 1;
        }

        // Append count and character to result
        result.push_str(&count.to_string());
        result.push(current_char);

        // Move to the next different character
        i += count;
    }

    result
}

// Export the component
bindings::export!(Component with_types_in bindings);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rle_basic() {
        assert_eq!(run_length_encode("aaabbc"), "3a2b1c");
        assert_eq!(run_length_encode("aaa"), "3a");
        assert_eq!(run_length_encode("abc"), "1a1b1c");
    }

    #[test]
    fn test_rle_empty() {
        assert_eq!(run_length_encode(""), "");
    }

    #[test]
    fn test_rle_single_char() {
        assert_eq!(run_length_encode("a"), "1a");
    }

    #[test]
    fn test_rle_long_sequence() {
        assert_eq!(run_length_encode("aaaaaaaaaa"), "10a");
    }

    #[test]
    fn test_component_process_basic() {
        let input = b"aaabbc";
        let result = Component::process(input.to_vec());

        assert!(result.is_ok());
        let output = result.unwrap();
        let output_str = std::str::from_utf8(&output).unwrap();
        assert_eq!(output_str, "3a2b1c");
    }

    #[test]
    fn test_component_process_empty() {
        let input = b"";
        let result = Component::process(input.to_vec());

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.len(), 0);
    }

    #[test]
    fn test_component_process_invalid_utf8() {
        let invalid_utf8 = vec![0xC3, 0x28]; // Invalid UTF-8 sequence
        let result = Component::process(invalid_utf8);

        assert!(result.is_err());
        match result.unwrap_err() {
            ProcessingError::InvalidInput(_) => {}, // Expected
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_component_process_non_ascii() {
        let input = "a😀b".as_bytes().to_vec();
        let result = Component::process(input);

        assert!(result.is_err());
        match result.unwrap_err() {
            ProcessingError::InvalidInput(msg) => {
                assert_eq!(msg, "Non-ASCII characters are not supported");
            }
            _ => panic!("Expected InvalidInput error for non-ASCII"),
        }
    }

    #[test]
    fn test_component_process_too_large() {
        let input = vec![b'a'; (MAX_INPUT_SIZE + 1) as usize];
        let result = Component::process(input);

        assert!(result.is_err());
        match result.unwrap_err() {
            ProcessingError::InputTooLarge(size) => {
                assert_eq!(size, (MAX_INPUT_SIZE + 1));
            }
            _ => panic!("Expected InputTooLarge error"),
        }
    }
}
