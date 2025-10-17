// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! Run Length Encoding WASM Component - Component Model Interface
//! 
//! This component implements standard Run Length Encoding (RLE) compression
//! for text input using the WebAssembly Component Model interface.
//! It exports:
//! - process: func(input: list<u8>) -> result<list<u8>, processing-error>
//!
//! Memory management is handled automatically by wit-bindgen!

// Generate bindings from the WIT file
wit_bindgen::generate!({
    world: "dagwood-component",
    path: "../../../wit/versions/v1.0.0"
});

use exports::dagwood::component::processing_node::{
    Guest, ProcessingError,
};

struct Component;

impl Guest for Component {
    fn process(input: Vec<u8>) -> Result<Vec<u8>, ProcessingError> {
        // Convert input bytes to string
        let input_str = match std::str::from_utf8(&input) {
            Ok(s) => s,
            Err(e) => {
                return Err(ProcessingError::InvalidInput(
                    format!("Invalid UTF-8: {}", e)
                ));
            }
        };
        
        // Perform RLE encoding
        let encoded = run_length_encode(input_str);
        
        // Return as bytes
        Ok(encoded.into_bytes())
    }
}

/// Performs Run Length Encoding on the input string.
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
export!(Component);


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
}
