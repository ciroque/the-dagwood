// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! WASM binary encoding detection
//!
//! This module provides spec-compliant detection of WASM binary formats using wasmparser.
//! It distinguishes between modern Component Model components and classic core WASM modules.

use crate::backends::wasm::error::{WasmError, WASM_UNSUPPORTED_ENCODING};

use wasmparser::{Encoding, Parser, Payload};

/// Represents supported WebAssembly component types
///
/// - `Wit`: Modern Component Model with WIT interfaces (binary version 2+)
/// - `CStyle`: Classic core WASM modules with C-style interfaces (version 1)
///
/// Note: Legacy Preview 1 components (version 1 + "component" custom section) are
/// not represented here as they are unsupported and rejected with an error.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ComponentType {
    /// Modern Component Model with WIT interface (binary version 2+)
    Wit,
    /// Classic core WASM module with C-style interface (version 1, no component section)
    CStyle,
}

impl ComponentType {
    /// Returns true if this is a WIT Component Model binary
    #[inline]
    pub fn is_wit(self) -> bool {
        matches!(self, Self::Wit)
    }

    /// Returns true if this is a C-style core WASM module
    #[inline]
    pub fn is_cstyle(self) -> bool {
        matches!(self, Self::CStyle)
    }
}

/// Detects the component type of a WebAssembly binary by inspecting its version header
/// and (for version 1) custom sections.
///
/// This function uses `wasmparser` to perform a spec-compliant parse, returning
/// `ComponentType::Wit` or `ComponentType::CStyle`. It **rejects** legacy
/// Preview 1 components with an error, as they are unsupported.
///
/// # Errors
/// Returns an error if:
/// - The input is empty, truncated, or otherwise invalid per the WASM spec
/// - A legacy Preview 1 component is detected (unsupported)
///
pub fn detect_component_type(bytes: &[u8]) -> Result<ComponentType, WasmError> {
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

    let encoding =
        encoding.ok_or_else(|| WasmError::InvalidWasmBinary("Invalid WASM binary".to_string()))?;

    match encoding {
        Encoding::Component => Ok(ComponentType::Wit),
        Encoding::Module if has_component_section => Err(WasmError::UnsupportedEncoding(
            WASM_UNSUPPORTED_ENCODING.to_string(),
        )),
        Encoding::Module => Ok(ComponentType::CStyle),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        let empty: &[u8] = &[];
        let result = detect_component_type(empty);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_input() {
        let bad = b"\x00\x00\x00\x00\x00\x00\x00\x00";
        let result = detect_component_type(bad);
        assert!(result.is_err());
    }

    #[test]
    fn test_component_type_helper_methods() {
        assert!(ComponentType::Wit.is_wit());
        assert!(!ComponentType::Wit.is_cstyle());

        assert!(ComponentType::CStyle.is_cstyle());
        assert!(!ComponentType::CStyle.is_wit());
    }
}
