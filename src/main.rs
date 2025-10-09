// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use std::env;
use std::time::Instant;
use std::collections::HashMap;
use std::io::{self, Write};
use the_dagwood::config::{RuntimeBuilder, DependencyGraph, EntryPoints, load_and_validate_config};
use the_dagwood::proto::processor_v1::ProcessorRequest;
use the_dagwood::proto::processor_v1::processor_response::Outcome;

/// Get the default concurrency level based on system capabilities
/// 
/// Returns the number of available CPU cores, falling back to 4 if detection fails.
/// This provides a sensible default for concurrent processor execution.
fn default_concurrency() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

const UNKNOWN_KEY: &str = "unknown";

/// Demo configuration with description and learning objectives
struct DemoConfig {
    file: &'static str,
    title: &'static str,
    description: &'static str,
    learning_objectives: Vec<&'static str>,
}

/// Wait for user to press Enter with a custom prompt
fn wait_for_keypress(prompt: &str) {
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    
    // Check for demo mode
    if args.len() >= 2 && args[1] == "--demo-mode" {
        run_guided_demo().await;
        return;
    }
    
    if args.len() < 3 {
        eprintln!("Usage: {} <config1.yaml> [config2.yaml ...] <input_text>", args[0]);
        eprintln!("       {} --demo-mode", args[0]);
        eprintln!("Example: {} configs/strategy-reactive-demo.yaml \"hello world\"", args[0]);
        eprintln!("Example: {} configs/strategy-workqueue-demo.yaml configs/strategy-reactive-demo.yaml \"test input\"", args[0]);
        eprintln!("Demo:    {} --demo-mode", args[0]);
        std::process::exit(1);
    }
    
    // The last argument is the input text
    let input_text = &args[args.len() - 1];
    // All arguments except the first (program name) and last (input) are config files
    let config_files = &args[1..args.len() - 1];
    
    println!("🚀 DAGwood Execution Strategy Demo");
    println!("═══════════════════════════════════");
    println!("Input: \"{}\"", input_text);
    println!("Config files: {:?}", config_files);
    println!();
    
    for (i, config_file) in config_files.iter().enumerate() {
        if i > 0 {
            println!("\n{}", "─".repeat(80)); // This is neat, the `repeat` function to generate a string of a certain length
        }
        
        match run_single_config(config_file, input_text).await {
            Ok(_) => {},
            Err(e) => {
                eprintln!("❌ Failed to execute {}: {}", config_file, e);
            }
        }
    }
    
    println!("\n🎉 Demo complete!");
}

/// Run the guided interactive demo with progressive complexity
async fn run_guided_demo() {
    let demo_configs = vec![
        DemoConfig {
            file: "docs/demo/configs/01-hello-world.yaml",
            title: "Hello World: Single Processor",
            description: "The simplest possible DAG with one processor and no dependencies",
            learning_objectives: vec![
                "Basic Rust ownership patterns in processor execution",
                "Simple async/await usage with tokio",
                "ProcessorRequest and ProcessorResponse structures",
                "Entry point detection in DAG execution"
            ],
        },
        DemoConfig {
            file: "docs/demo/configs/02-text-pipeline.yaml",
            title: "Text Pipeline: Linear Chain",
            description: "A sequence of processors showing data flow and chaining",
            learning_objectives: vec![
                "Data flow chaining between processors",
                "Dependency resolution and topological ordering",
                "Rust Result<T, E> error handling patterns",
                "Arc and Mutex for shared state management"
            ],
        },
        DemoConfig {
            file: "docs/demo/configs/03-diamond-analysis.yaml",
            title: "Diamond Analysis: Parallel Execution",
            description: "Classic diamond dependency pattern with parallel analysis processors",
            learning_objectives: vec![
                "Parallel execution with tokio async tasks",
                "Canonical payload architecture (Transform vs Analyze)",
                "Metadata collection and merging strategies",
                "Race condition prevention in concurrent execution"
            ],
        },
        DemoConfig {
            file: "docs/demo/configs/04-wasm-integration.yaml",
            title: "WASM Integration: Sandboxed Processing",
            description: "WASM processor integration with security sandboxing",
            learning_objectives: vec![
                "WASM module loading and execution with wasmtime",
                "Memory management across WASM boundary",
                "Security sandboxing and isolation patterns",
                "Multi-backend processor architecture"
            ],
        },
        DemoConfig {
            file: "docs/demo/configs/05-complex-workflow.yaml",
            title: "Complex Workflow: Multi-Backend Pipeline",
            description: "Advanced DAG with multiple backends and execution strategies",
            learning_objectives: vec![
                "Level-by-Level vs Work Queue execution strategies",
                "Mixed local and WASM processor coordination",
                "Advanced error handling with failure strategies",
                "Production-ready workflow orchestration patterns"
            ],
        },
    ];

    println!("🦀 The DAGwood Project - Interactive Demo");
    println!("═══════════════════════════════════════════");
    println!();
    println!("Welcome to an interactive demonstration of The DAGwood Project!");
    println!("This demo showcases our four primary learning objectives:");
    println!();
    println!("🦀 1. Learn Rust - Ownership, async/await, traits, and error handling");
    println!("🔄 2. Learn DAG Execution Strategies - Work Queue, Level-by-Level, Reactive");
    println!("🧩 3. Learn WASM Components - Security sandboxing and multi-language support");
    println!("🤖 4. Use AI Tools - How AI assistance accelerated development");
    println!();
    println!("We'll progress through 5 examples of increasing complexity:");
    for (i, config) in demo_configs.iter().enumerate() {
        println!("  {}. {}", i + 1, config.title);
    }
    println!();
    
    wait_for_keypress("Press Enter to begin the demo... ");
    
    let input_text = "hello world";
    
    for (i, demo_config) in demo_configs.iter().enumerate() {
        println!("\n{}", "═".repeat(80));
        println!("Demo {}: {}", i + 1, demo_config.title);
        println!("{}", "═".repeat(80));
        println!();
        println!("📖 Description:");
        println!("   {}", demo_config.description);
        println!();
        println!("🎯 Learning Objectives:");
        for objective in &demo_config.learning_objectives {
            println!("   • {}", objective);
        }
        println!();
        
        wait_for_keypress(&format!("Press Enter to run Demo {} ({})... ", i + 1, demo_config.title));
        
        match run_single_config(demo_config.file, input_text).await {
            Ok(_) => {
                println!("\n✅ Demo {} completed successfully!", i + 1);
            },
            Err(e) => {
                println!("\n❌ Demo {} failed: {}", i + 1, e);
                println!("   This might be expected if WASM modules aren't built yet.");
            }
        }
        
        if i < demo_configs.len() - 1 {
            println!();
            wait_for_keypress("Press Enter to continue to the next demo... ");
        }
    }
    
    println!("\n{}", "═".repeat(80));
    println!("🎉 Demo Complete - Thank You!");
    println!("{}", "═".repeat(80));
    println!();
    println!("You've seen The DAGwood Project demonstrate:");
    println!("• 🦀 Rust ownership, async/await, and error handling patterns");
    println!("• 🔄 Multiple DAG execution strategies (Work Queue, Level-by-Level)");
    println!("• 🧩 WASM component integration with security sandboxing");
    println!("• 🤖 AI-assisted development accelerating complex implementations");
    println!();
    println!("Next Steps:");
    println!("• 📚 Explore the full mdBook presentation: cd docs/demo && mdbook serve");
    println!("• 🔍 Examine the source code and ADRs for architectural decisions");
    println!("• 🚀 Try building your own processors and DAG configurations");
    println!("• 🤝 Contribute to the project or adapt it for your use cases");
    println!();
    println!("Thank you for exploring The DAGwood Project!");
}

async fn run_single_config(config_file: &str, input_text: &str) -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    
    // Load configuration
    let config = load_and_validate_config(config_file)?;

    // Build runtime components from configuration
    let (processors, executor, failure_strategy) = RuntimeBuilder::from_config(&config);
    
    // Build dependency graph and entry points from config
    let mut graph_map = HashMap::new();
    let mut entry_points_vec = Vec::new();
    
    // Build dependency graph from processor configurations
    for processor_config in &config.processors {
        // Initialize processor in graph
        if !graph_map.contains_key(&processor_config.id) {
            graph_map.insert(processor_config.id.clone(), Vec::new());
        }
        
        // If this processor has no dependencies, it's an entry point
        if processor_config.depends_on.is_empty() {
            entry_points_vec.push(processor_config.id.clone());
        } else {
            // Add this processor as a dependent of its dependencies
            for dependency_id in &processor_config.depends_on {
                graph_map.entry(dependency_id.clone())
                    .or_insert_with(Vec::new)
                    .push(processor_config.id.clone());
            }
        }
    }
    
    let dependency_graph = DependencyGraph(graph_map);
    let entry_points = EntryPoints(entry_points_vec);

    use the_dagwood::proto::processor_v1::{PipelineMetadata, ProcessorMetadata};
    let request_metadata = HashMap::from([{
        ("initial_context".to_string(), ProcessorMetadata {
            metadata: HashMap::from([
                ("config_file".to_string(), config_file.to_string()),
                ("hostname".to_string(), std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_string())),
                (
                    "input_text".to_string(),
                    input_text.to_string(),
                )
            ]),
        })
    }]);

    // Prepare input and pipeline metadata
    let pipeline_metadata = PipelineMetadata {
        metadata: request_metadata,
    };
    let input = ProcessorRequest {
        payload: input_text.as_bytes().to_vec(),
    };
    
    println!("📋 Configuration: {}", config_file);
    println!("🔧 Strategy: {:?}", config.strategy);
    println!("⚙️  Max Concurrency: {}", 
        config.executor_options.max_concurrency.unwrap_or_else(default_concurrency)
    );
    println!("🛡️  Failure Strategy: {:?}", config.failure_strategy);
    
    // Execute the DAG
    let execution_start = Instant::now();
    let (results, final_pipeline_metadata) = executor.execute_with_strategy(
        processors,
        dependency_graph,
        entry_points,
        input,
        pipeline_metadata,
        failure_strategy,
    ).await?;
    let execution_time = execution_start.elapsed();
    
    // Display results
    println!("\n📊 Execution Results:");
    println!("⏱️  Execution Time: {:?}", execution_time);
    println!("🔢 Processors Executed: {}", results.len());
    
    // Show processor chain and transformations
    println!("\n🔄 Processor Chain:");
    
    // Try to determine execution order from dependencies (simple heuristic)
    let mut ordered_processors = Vec::new();
    
    // Find entry points (processors with no dependencies)
    for processor_config in &config.processors {
        if processor_config.depends_on.is_empty() {
            ordered_processors.push(processor_config.id.clone());
        }
    }
    
    // Add remaining processors in dependency order (simple approach)
    let mut added = std::collections::HashSet::new();
    for proc in &ordered_processors {
        added.insert(proc.clone());
    }
    
    let mut changed = true;
    while changed {
        changed = false;
        for processor_config in &config.processors {
            if !added.contains(&processor_config.id) {
                // Check if all dependencies are already added
                let all_deps_added = processor_config.depends_on.iter()
                    .all(|dep| added.contains(dep));
                
                if all_deps_added {
                    ordered_processors.push(processor_config.id.clone());
                    added.insert(processor_config.id.clone());
                    changed = true;
                }
            }
        }
    }

    for (i, processor_id) in ordered_processors.iter().enumerate() {
        if let Some(result) = results.get(processor_id) {
            let output = if let Some(Outcome::NextPayload(payload)) = &result.outcome {
                String::from_utf8_lossy(payload.as_slice()).to_string()
            } else {
                "[No output]".to_string()
            };
            
            println!("  {}. {} → \"{}\"", i + 1, processor_id, output);
            
            // Show metadata if present
            if let Some(pipeline_metadata) = &result.metadata {
                println!("     📝 Metadata: {} entries", pipeline_metadata.metadata.len());
                for (key, metadata) in pipeline_metadata.metadata.iter().take(3) { // Show the first 3 metadata entries
                    if !metadata.metadata.is_empty() {
                        let sample_key = metadata.metadata.keys().next().map(|k| k.as_str()).unwrap_or(UNKNOWN_KEY);
                        println!("        • {}: {} keys (e.g., {})", key, metadata.metadata.len(), sample_key);
                    }
                }
                if pipeline_metadata.metadata.len() > 3 {
                    println!("        • ... and {} more", pipeline_metadata.metadata.len() - 3);
                }
            } else {
                println!("     📝 Metadata: no entries");
            }
        }
    }
    
    // Final transformation summary
    if let Some(final_processor) = ordered_processors.last() {
        if let Some(final_result) = results.get(final_processor) {
            if let Some(Outcome::NextPayload(final_payload)) = &final_result.outcome {
                let final_output = String::from_utf8_lossy(final_payload);
                println!("\n🎯 Final Transformation:");
                println!("   Input:  \"{}\"", input_text);
                println!("   Output: \"{}\"", final_output);
            }

            // Show accumulated pipeline metadata
            if final_pipeline_metadata.metadata.is_empty() {
                println!("   No metadata");
            } else {
                println!("   Pipeline Metadata:");
                for (processor_name, metadata) in final_pipeline_metadata.metadata.iter() {
                    println!("   {}:", processor_name);
                    for (key, value) in metadata.metadata.iter() {
                        println!("      • {}: {}", key, value);
                    }
                }
            }
        }
    }
    
    let total_time = start_time.elapsed();
    println!("\n⏱️  Total Time (including config load): {:?}", total_time);
    
    Ok(())
}