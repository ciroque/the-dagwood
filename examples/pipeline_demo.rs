use std::collections::HashMap;
use the_dagwood::backends::local::LocalProcessorRegistry;
use the_dagwood::proto::processor_v1::ProcessorRequest;

/// Demo showing a simple pipeline: change text case -> reverse text
async fn run_pipeline_demo() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== DAGwood Local Backend Pipeline Demo ===\n");
    
    // Create the local processor registry
    let registry = LocalProcessorRegistry::new();
    
    // Input text for our pipeline
    let input_text = "Hello, World! This is a test.";
    println!("Input: '{}'", input_text);
    
    // Step 1: Change text case to uppercase
    let uppercase_processor = registry.get("change_text_case_upper")
        .ok_or("Uppercase processor not found")?;
    
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
    let reverse_processor = registry.get("reverse_text")
        .ok_or("Reverse processor not found")?;
    
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
    let token_counter = registry.get("token_counter")
        .ok_or("Token counter not found")?;
    
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
    let word_freq = registry.get("word_frequency_analyzer")
        .ok_or("Word frequency analyzer not found")?;
    
    let freq_request = ProcessorRequest {
        payload: input_text.as_bytes().to_vec(),
        metadata: HashMap::new(),
    };
    
    let freq_response = word_freq.process(freq_request).await;
    if let Some(the_dagwood::proto::processor_v1::processor_response::Outcome::NextPayload(payload)) = freq_response.outcome {
        let freq_result = String::from_utf8(payload)?;
        println!("Word frequency result: {}", freq_result);
    }
    
    // Prefix/Suffix Adder
    let bracket_adder = registry.get("add_brackets")
        .ok_or("Bracket adder not found")?;
    
    let bracket_request = ProcessorRequest {
        payload: "test".as_bytes().to_vec(),
        metadata: HashMap::new(),
    };
    
    let bracket_response = bracket_adder.process(bracket_request).await;
    if let Some(the_dagwood::proto::processor_v1::processor_response::Outcome::NextPayload(payload)) = bracket_response.outcome {
        let bracket_result = String::from_utf8(payload)?;
        println!("Bracket adder result: '{}'", bracket_result);
    }
    
    println!("\n=== Available Processors ===");
    let processors = registry.list_processors();
    for processor_id in processors {
        println!("- {}", processor_id);
    }
    
    println!("\nDemo completed successfully!");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_pipeline_demo().await
}
