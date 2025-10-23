// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! WASM binary encoding detection
//!
//! This module provides spec-compliant detection of WASM binary formats using wasmparser.
//! It distinguishes between modern Component Model components and classic core WASM modules.

use crate::backends::wasm::error::{WasmError, WASM_UNSUPPORTED_ENCODING};

use wasmparser::{Encoding, Parser, Payload};

/// Represents supported WebAssembly binary encodings
/// 
/// - `ComponentModel`: Modern Component Model (binary version 2+)
/// - `Classic`: Core WebAssembly modules (version 1, no component indicators)
/// 
/// Note: Legacy Preview 1 components (version 1 + "component" custom section) are
/// not represented here as they are unsupported and rejected with an error.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WasmEncoding {
    /// Modern Component Model (binary version 2+)
    ComponentModel,
    /// Classic core WASM module (version 1, no component section)
    Classic,
}

impl WasmEncoding {
    /// Returns true if this is a Component Model binary
    #[inline]
    pub fn is_component_model(self) -> bool {
        matches!(self, Self::ComponentModel)
    }

    /// Returns true if this is a classic core WASM module
    #[inline]
    pub fn is_classic(self) -> bool {
        matches!(self, Self::Classic)
    }
}

/// Determines the encoding of a WebAssembly binary by inspecting its version header
/// and (for version 1) custom sections.
/// 
/// This function uses `wasmparser` to perform a spec-compliant parse, returning
/// `WasmEncoding::ComponentModel` or `WasmEncoding::Classic`. It **rejects** legacy
/// Preview 1 components with an error, as they are unsupported.
/// 
/// # Errors
/// Returns an error if:
/// - The input is empty, truncated, or otherwise invalid per the WASM spec
/// - A legacy Preview 1 component is detected (unsupported)
/// 
pub fn wasm_encoding(bytes: &[u8]) -> Result<WasmEncoding, WasmError> {
    let parser = Parser::new(0);
    let mut encoding = None;
    let mut has_component_section = false;

    for payload in parser.parse_all(bytes) {
        let payload = payload?;
        match payload {
            Payload::Version { encoding: enc, .. } => {
                encoding = Some(enc);
            }
            Payload::CustomSection(reader) if reader.name() == "component" => {
                has_component_section = true;
            }
            _ => {}
        }
    }

    let encoding = encoding.ok_or_else(|| {
        WasmError::InvalidWasmBinary("Invalid WASM binary".to_string())
    })?;

    match encoding {
        Encoding::Component => Ok(WasmEncoding::ComponentModel),
        Encoding::Module if has_component_section => Err(WasmError::UnsupportedEncoding(WASM_UNSUPPORTED_ENCODING.to_string())),
        Encoding::Module => Ok(WasmEncoding::Classic),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        let empty: &[u8] = &[];
        let result = wasm_encoding(empty);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_input() {
        let bad = b"\x00\x00\x00\x00\x00\x00\x00\x00";
        let result = wasm_encoding(bad);
        assert!(result.is_err());
    }

    #[test]
    fn test_encoding_helper_methods() {
        assert!(WasmEncoding::ComponentModel.is_component_model());
        assert!(!WasmEncoding::ComponentModel.is_classic());
        
        assert!(WasmEncoding::Classic.is_classic());
        assert!(!WasmEncoding::Classic.is_component_model());
    }
}
