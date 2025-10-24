// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! Local (in-process) processor backend for native Rust implementations.
//!
//! This module provides a collection of built-in processors that execute directly in the
//! Rust process without sandboxing or external communication. These processors offer
//! zero-overhead execution and are ideal for trusted code, testing, and rapid prototyping.
//!
//! # Architecture
//!
//! The local backend follows a factory pattern:
//! ```text
//! ProcessorConfig → LocalProcessorFactory → Processor Instance
//! ```
//!
//! ## Components
//! - **LocalProcessorFactory**: Creates processor instances from configuration
//! - **Processors**: Individual processor implementations (text manipulation, analysis)
//!
//! # Available Processors
//!
//! ## Text Transformation
//! - **change_text_case_upper**: Convert text to UPPERCASE
//! - **change_text_case_lower**: Convert text to lowercase
//! - **change_text_case_proper**: Convert text to Proper Case
//! - **change_text_case_title**: Convert text to Title Case
//! - **reverse_text**: Reverse character order
//! - **prefix_suffix_adder**: Add configurable prefix and suffix
//!
//! ## Text Analysis
//! - **token_counter**: Count characters, words, and lines
//! - **word_frequency_analyzer**: Analyze word frequency distribution
//!
//! # Performance Characteristics
//!
//! - **Overhead**: Zero - direct function calls
//! - **Memory**: Minimal - no serialization or IPC
//! - **Latency**: Sub-microsecond for simple operations
//! - **Throughput**: Limited only by CPU and algorithm complexity
//!
//! # Examples
//!
//! ## Creating a Processor from Configuration
//! ```rust,ignore
//! use the_dagwood::backends::local::LocalProcessorFactory;
//! use the_dagwood::config::{ProcessorConfig, BackendType};
//! use std::collections::HashMap;
//!
//! let config = ProcessorConfig {
//!     id: "uppercase".to_string(),
//!     backend: BackendType::Local,
//!     processor: Some("change_text_case_upper".to_string()),
//!     endpoint: None,
//!     module: None,
//!     depends_on: vec![],
//!     options: HashMap::new(),
//! };
//! let processor = LocalProcessorFactory::create_processor(&config)?;
//! # Ok::<(), String>(())
//! ```
//!
//! ## Using a Processor with Custom Options
//! ```rust,no_run
//! use the_dagwood::backends::local::LocalProcessorFactory;
//! use the_dagwood::config::{ProcessorConfig, BackendType};
//! use std::collections::HashMap;
//! use serde_yaml::Value;
//!
//! let mut options = HashMap::new();
//! options.insert("prefix".to_string(), Value::String("[".to_string()));
//! options.insert("suffix".to_string(), Value::String("]".to_string())); 
//!
//! let config = ProcessorConfig {
//!     id: "brackets".to_string(),
//!     backend: BackendType::Local,
//!     processor: Some("prefix_suffix_adder".to_string()),
//!     endpoint: None,
//!     module: None,
//!     depends_on: vec![],
//!     options,
//! };
//!
//! let processor = LocalProcessorFactory::create_processor(&config)?;
//! # Ok::<(), String>(())
//! ```

pub mod factory;
pub mod processors;

pub use factory::LocalProcessorFactory;
pub use processors::*;
