use std::collections::HashMap;
use the_dagwood::backends::local::LocalProcessorFactory;
use the_dagwood::config::{ProcessorConfig, BackendType};
use the_dagwood::proto::processor_v1::ProcessorRequest;

/// Demo showing a simple pipeline: change text case -> reverse text
async fn run_pipeline_demo() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== DAGwood Local Backend Pipeline Demo ===\n");
    
    // Helper function to create a processor config
    let create_config = |id: &str, impl_name: &str| ProcessorConfig {
        id: id.to_string(),
        backend: BackendType::Local,
        processor: Some(impl_name.to_string()),
        endpoint: None,
        module: None,
        depends_on: vec![],
        options: HashMap::new(),
    };
    
    // Input text for our pipeline
    let input_text = "Hello, World! This is a test.";
    println!("Input: '{}'", input_text);
    
    // Step 1: Change text case to uppercase
    let uppercase_config = create_config("uppercase", "change_text_case_upper");
    let uppercase_processor = LocalProcessorFactory::create_processor(&uppercase_config)
        .map_err(|e| format!("Failed to create uppercase processor: {}", e))?;
    
    let request1 = ProcessorRequest {
        payload: input_text.as_bytes().to_vec(),
        metadata: HashMap::new(),
    };
    
    let response1 = uppercase_processor.process(request1).await;
    let uppercase_result = match response1.outcome {
        Some(the_dagwood::proto::processor_v1::processor_response::Outcome::NextPayload(payload)) => {
            String::from_utf8(payload)?
        }
        Some(the_dagwood::proto::processor_v1::processor_response::Outcome::Error(err)) => {
            return Err(format!("Uppercase processor error: {}", err.message).into());
        }
        None => return Err("No outcome from uppercase processor".into()),
    };
    
    println!("After uppercase: '{}'", uppercase_result);
    
    // Step 2: Reverse the text
    let reverse_config = create_config("reverse", "reverse_text");
    let reverse_processor = LocalProcessorFactory::create_processor(&reverse_config)
        .map_err(|e| format!("Failed to create reverse processor: {}", e))?;
    
    let request2 = ProcessorRequest {
        payload: uppercase_result.as_bytes().to_vec(),
        metadata: HashMap::new(),
    };
    
    let response2 = reverse_processor.process(request2).await;
    let final_result = match response2.outcome {
        Some(the_dagwood::proto::processor_v1::processor_response::Outcome::NextPayload(payload)) => {
            String::from_utf8(payload)?
        }
        Some(the_dagwood::proto::processor_v1::processor_response::Outcome::Error(err)) => {
            return Err(format!("Reverse processor error: {}", err.message).into());
        }
        None => return Err("No outcome from reverse processor".into()),
    };
    
    println!("Final result: '{}'", final_result);
    
    // Demonstrate other processors
    println!("\n=== Other Processor Demos ===");
    
    // Token Counter
    let token_config = create_config("token_counter", "token_counter");
    let token_counter = LocalProcessorFactory::create_processor(&token_config)
        .map_err(|e| format!("Failed to create token counter: {}", e))?;
    
    let token_request = ProcessorRequest {
        payload: input_text.as_bytes().to_vec(),
        metadata: HashMap::new(),
    };
    
    let token_response = token_counter.process(token_request).await;
    if let Some(the_dagwood::proto::processor_v1::processor_response::Outcome::NextPayload(payload)) = token_response.outcome {
        let token_result = String::from_utf8(payload)?;
        println!("Token count result: {}", token_result);
    }
    
    // Word Frequency Analyzer
    let word_freq_config = create_config("word_freq", "word_frequency_analyzer");
    let word_freq = LocalProcessorFactory::create_processor(&word_freq_config)
        .map_err(|e| format!("Failed to create word frequency analyzer: {}", e))?;
    
    let freq_request = ProcessorRequest {
        payload: input_text.as_bytes().to_vec(),
        metadata: HashMap::new(),
    };
    
    let freq_response = word_freq.process(freq_request).await;
    if let Some(the_dagwood::proto::processor_v1::processor_response::Outcome::NextPayload(payload)) = freq_response.outcome {
        let freq_result = String::from_utf8(payload)?;
        println!("Word frequency result: {}", freq_result);
    }
    
    // Prefix/Suffix Adder (creates with default brackets)
    let bracket_config = create_config("bracket_adder", "prefix_suffix_adder");
    let bracket_adder = LocalProcessorFactory::create_processor(&bracket_config)
        .map_err(|e| format!("Failed to create bracket adder: {}", e))?;
    
    let bracket_request = ProcessorRequest {
        payload: "test".as_bytes().to_vec(),
        metadata: HashMap::new(),
    };
    
    let bracket_response = bracket_adder.process(bracket_request).await;
    if let Some(the_dagwood::proto::processor_v1::processor_response::Outcome::NextPayload(payload)) = bracket_response.outcome {
        let bracket_result = String::from_utf8(payload)?;
        println!("Bracket adder result: '{}'", bracket_result);
    }
    
    println!("\n=== Available Processor Implementations ===");
    let implementations = LocalProcessorFactory::list_available_implementations();
    for impl_name in implementations {
        println!("- {}", impl_name);
    }
    
    println!("\nDemo completed successfully!");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_pipeline_demo().await
}
