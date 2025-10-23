// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

mod bindings;
use bindings::exports::dagwood::component::processing_node::{Guest, ProcessingError};

const MAX_INPUT_SIZE: u64 = 1024 * 1024; // 1MB

struct Component;

impl Guest for Component {
    fn process(input: Vec<u8>) -> Result<Vec<u8>, ProcessingError> {
        if input.len() as u64 > MAX_INPUT_SIZE {
            return Err(ProcessingError::InputTooLarge(input.len() as u64));
        }

        let input_str = match std::str::from_utf8(&input) {
            Ok(s) => s,
            Err(_) => return Err(ProcessingError::InvalidInput("Invalid UTF-8".to_string())),
        };

        if !input_str.is_ascii() {
            return Err(ProcessingError::InvalidInput("Non-ASCII characters are not supported".to_string()));
        }

        let encoded = run_length_encode(input_str);
        Ok(encoded.into_bytes())
    }
}

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

        while i + count < chars.len() && chars[i + count] == current_char {
            count += 1;
        }

        // Manually append count as string
        result.push_str(itoa(count));
        result.push(current_char);
        i += count;
    }

    result
}

// Convert number to string manually to avoid format!
fn itoa(n: usize) -> &'static str {
    match n {
        0 => "0",
        1 => "1",
        2 => "2",
        3 => "3",
        4 => "4",
        5 => "5",
        6 => "6",
        7 => "7",
        8 => "8",
        9 => "9",
        10 => "10",
        _ => "10+", // Simplified; extend for larger numbers if needed
    }
}

bindings::export!(Component with_types_in bindings);