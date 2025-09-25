use std::collections::HashMap;
use std::env;
use the_dagwood::config::{load_and_validate_config, build_dag_runtime};
use the_dagwood::proto::ProcessorRequest;
use the_dagwood::traits::{ProcessorMap, DependencyGraph, EntryPoints};

/// Demo showing Phase 2, Step 3: Run a trivial pipeline using YAML configuration
/// Usage: cargo run --example yaml_pipeline_demo <config_file> <input_text>
async fn run_yaml_pipeline_demo(config_file: String, input_text: String) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== YAML-Configured Pipeline Demo ===\n");
    
    // Load the specified pipeline configuration
    println!("Loading configuration from {}...", config_file);
    let config = load_and_validate_config(&config_file)
        .map_err(|e| format!("Failed to load configuration: {}", e))?;
    
    println!("Configuration loaded successfully!");
    println!("- Strategy: {:?}", config.strategy);
    println!("- Failure Strategy: {:?}", config.failure_strategy);
    println!("- Max Concurrency: {:?}", config.executor_options.max_concurrency);
    println!("- Processors: {}", config.processors.len());
    
    // Build the DAG runtime from configuration
    println!("\nBuilding DAG runtime...");
    let (processors, executor, failure_strategy) = build_dag_runtime(&config);
    
    println!("DAG runtime built successfully!");
    println!("- Processors registered: {}", processors.len());
    println!("- Failure strategy: {:?}", failure_strategy);
    
    // Create the dependency graph from configuration
    // The Work Queue executor expects a graph where each processor maps to its dependents
    let mut dependency_graph = HashMap::new();
    
    // Initialize all processors with empty dependent lists
    for processor_config in &config.processors {
        dependency_graph.insert(processor_config.id.clone(), Vec::new());
    }
    
    // Build the forward dependency graph (processor -> list of dependents)
    for processor_config in &config.processors {
        for dependency_id in &processor_config.depends_on {
            dependency_graph.entry(dependency_id.clone())
                .or_insert_with(Vec::new)
                .push(processor_config.id.clone());
        }
    }
    
    // Find entry points (processors with no dependencies)
    let entrypoints: Vec<String> = config.processors
        .iter()
        .filter(|p| p.depends_on.is_empty())
        .map(|p| p.id.clone())
        .collect();
    
    println!("\nDependency graph:");
    for processor_config in &config.processors {
        if processor_config.depends_on.is_empty() {
            println!("- {} (entry point)", processor_config.id);
        } else {
            println!("- {} depends on: {:?}", processor_config.id, processor_config.depends_on);
        }
    }
    
    // Use the provided input text for our pipeline
    println!("\n=== Executing Pipeline ===");
    println!("Input: '{}'", input_text);
    
    // Create the input request
    let input_request = ProcessorRequest {
        payload: input_text.as_bytes().to_vec(),
        metadata: HashMap::new(),
    };
    
    // Execute the DAG with the configured failure strategy
    println!("Executing DAG with Work Queue executor...");
    let results = executor.execute_with_strategy(
        ProcessorMap::from(processors),
        DependencyGraph::from(dependency_graph),
        EntryPoints::from(entrypoints),
        input_request,
        failure_strategy,
    ).await?;
    
    println!("\n=== Pipeline Results ===");
    for (processor_id, response) in &results {
        if let Some(outcome) = &response.outcome {
            match outcome {
                the_dagwood::proto::processor_v1::processor_response::Outcome::NextPayload(payload) => {
                    let result_text = String::from_utf8_lossy(payload);
                    println!("- {}: '{}'", processor_id, result_text);
                }
                the_dagwood::proto::processor_v1::processor_response::Outcome::Error(err) => {
                    println!("- {}: ERROR - {}", processor_id, err.message);
                }
            }
        }
    }
    
    // Verify the expected pipeline flow
    println!("\n=== Pipeline Verification ===");
    if let Some(uppercase_response) = results.get("uppercase") {
        if let Some(the_dagwood::proto::processor_v1::processor_response::Outcome::NextPayload(payload)) = &uppercase_response.outcome {
            let uppercase_result = String::from_utf8_lossy(payload);
            println!("✓ Uppercase processor: '{}' -> '{}'", input_text, uppercase_result);
            
            if let Some(reverse_response) = results.get("reverse") {
                if let Some(the_dagwood::proto::processor_v1::processor_response::Outcome::NextPayload(payload)) = &reverse_response.outcome {
                    let final_result = String::from_utf8_lossy(payload);
                    println!("✓ Reverse processor: '{}' -> '{}'", uppercase_result, final_result);
                    
                    // Expected result: "hello world" -> "HELLO WORLD" -> "DLROW OLLEH"
                    let expected = "DLROW OLLEH";
                    if final_result == expected {
                        println!("✅ Pipeline completed successfully! Final result matches expected: '{}'", expected);
                    } else {
                        println!("❌ Pipeline result mismatch. Expected: '{}', Got: '{}'", expected, final_result);
                    }
                } else {
                    println!("❌ Reverse processor failed or returned no payload");
                }
            } else {
                println!("❌ Reverse processor not found in results");
            }
        } else {
            println!("❌ Uppercase processor failed or returned no payload");
        }
    } else {
        println!("❌ Uppercase processor not found in results");
    }
    
    // Identify and display the final result
    println!("\n=== Final Pipeline Result ===");
    
    // Find processors with no dependents (leaf nodes) - these are the final outputs
    let mut final_processors = Vec::new();
    for processor_config in &config.processors {
        let processor_id = &processor_config.id;
        let is_dependency = config.processors.iter()
            .any(|p| p.depends_on.contains(processor_id));
        
        if !is_dependency && results.contains_key(processor_id) {
            final_processors.push(processor_id);
        }
    }
    
    if final_processors.is_empty() {
        println!("No final processors found (all processors are dependencies of others)");
    } else if final_processors.len() == 1 {
        let final_processor_id = final_processors[0];
        if let Some(final_response) = results.get(final_processor_id) {
            if let Some(the_dagwood::proto::processor_v1::processor_response::Outcome::NextPayload(payload)) = &final_response.outcome {
                let final_result = String::from_utf8_lossy(payload);
                println!("Final result from '{}': '{}'", final_processor_id, final_result);
            } else {
                println!("Final processor '{}' did not produce a valid result", final_processor_id);
            }
        }
    } else {
        println!("Multiple final processors found:");
        for processor_id in &final_processors {
            if let Some(response) = results.get(*processor_id) {
                if let Some(the_dagwood::proto::processor_v1::processor_response::Outcome::NextPayload(payload)) = &response.outcome {
                    let result = String::from_utf8_lossy(payload);
                    println!("- {}: '{}'", processor_id, result);
                }
            }
        }
    }

    println!("\n=== Demo Summary ===");
    println!("✓ YAML configuration loaded and validated");
    println!("✓ DAG runtime built from configuration");
    println!("✓ Work Queue executor created with configured options");
    println!("✓ Pipeline executed with configured failure strategy");
    println!("✓ Results processed and verified");
    
    println!("\nYAML Pipeline Demo completed successfully!");
    Ok(())
}

fn print_usage() {
    println!("Usage: cargo run --example yaml_pipeline_demo <config_file> <input_text>");
    println!();
    println!("Arguments:");
    println!("  <config_file>  Path to the YAML configuration file");
    println!("  <input_text>   Input text to process through the pipeline");
    println!();
    println!("Examples:");
    println!("  cargo run --example yaml_pipeline_demo configs/simple-pipeline.yaml \"hello world\"");
    println!("  cargo run --example yaml_pipeline_demo configs/parallel-collection.yaml \"test input\"");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 3 {
        eprintln!("Error: Invalid number of arguments. Expected 2, got {}.", args.len() - 1);
        println!();
        print_usage();
        std::process::exit(1);
    }
    
    let config_file = args[1].clone();
    let input_text = args[2].clone();
    
    // Validate that the config file exists
    if !std::path::Path::new(&config_file).exists() {
        eprintln!("Error: Configuration file '{}' does not exist.", config_file);
        std::process::exit(1);
    }
    
    run_yaml_pipeline_demo(config_file, input_text).await
}
