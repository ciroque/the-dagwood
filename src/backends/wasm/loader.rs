// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! WASM file loading and validation
//!
//! This module handles reading WASM binaries from disk and performing basic
//! size validation. It does not parse or analyze the binary format - that's
//! the responsibility of the detector module.

use crate::backends::wasm::error::{WasmError, WasmResult};
use std::path::Path;

/// Maximum allowed size for WASM binaries (16 MB)
const MAX_WASM_SIZE: usize = 16 * 1024 * 1024;

/// Loads WASM bytes from a file and validates the size
///
/// This function reads the entire WASM binary into memory and checks that it
/// doesn't exceed the maximum allowed size. It does not parse or validate the
/// WASM format itself - use `wasm_encoding()` for that.
///
/// # Arguments
/// * `path` - Path to the WASM file to load
///
/// # Returns
/// * `Ok(Vec<u8>)` - The WASM binary bytes
/// * `Err(WasmError)` - If file cannot be read or size exceeds limit
pub fn load_wasm_bytes<P: AsRef<Path>>(path: P) -> WasmResult<Vec<u8>> {
    use crate::observability::messages::wasm::{ModuleLoaded, ModuleLoadFailed};

    let path = path.as_ref();
    let bytes = std::fs::read(path).map_err(|e| {
        let error = WasmError::IoError(e);
        tracing::error!(
            "{}",
            ModuleLoadFailed {
                module_path: &path.display().to_string(),
                error: &error,
            }
        );
        error
    })?;

    if bytes.len() > MAX_WASM_SIZE {
        let error = WasmError::ValidationError(format!(
            "WASM file too large: {} bytes (max: {} bytes)",
            bytes.len(),
            MAX_WASM_SIZE
        ));
        tracing::error!(
            "{}",
            ModuleLoadFailed {
                module_path: &path.display().to_string(),
                error: &error,
            }
        );
        return Err(error);
    }

    tracing::info!(
        "{}",
        ModuleLoaded {
            module_path: &path.display().to_string(),
            size_bytes: bytes.len(),
        }
    );

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_small_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"test wasm data";
        temp_file.write_all(test_data).unwrap();
        
        let result = load_wasm_bytes(temp_file.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_data);
    }

    #[test]
    fn test_file_too_large() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let large_data = vec![0u8; MAX_WASM_SIZE + 1];
        temp_file.write_all(&large_data).unwrap();

        let result = load_wasm_bytes(temp_file.path());
        assert!(result.is_err());

        if let Err(WasmError::ValidationError(msg)) = result {
            assert!(msg.contains("too large"));
            assert!(msg.contains(&format!("{}", MAX_WASM_SIZE + 1)));
        } else {
            panic!("Expected ValidationError for oversized file");
        }
    }

    #[test]
    fn test_nonexistent_file() {
        let result = load_wasm_bytes("/nonexistent/path/to/file.wasm");
        assert!(result.is_err());

        if let Err(WasmError::IoError(_)) = result {
        } else {
            panic!("Expected IoError for nonexistent file");
        }
    }

    #[test]
    fn test_max_size_boundary() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let max_data = vec![0u8; MAX_WASM_SIZE];
        temp_file.write_all(&max_data).unwrap();

        let result = load_wasm_bytes(temp_file.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), MAX_WASM_SIZE);
    }
}
