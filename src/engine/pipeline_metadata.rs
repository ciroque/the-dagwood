use std::collections::HashMap;
use crate::proto::processor_v1::{PipelineMetadata, ProcessorMetadata, ProcessorResponse};

impl PipelineMetadata {
    pub fn new() -> Self {
        Self {
            metadata: HashMap::new(),
        }
    }

    pub fn add_metadata(&mut self, processor_name: &str, key: &str, value: &str) {
        let processor_metadata = self.metadata.entry(processor_name.to_string()).or_insert(ProcessorMetadata {
            metadata: HashMap::new(),
        });
        processor_metadata.metadata.insert(key.to_string(), value.to_string());
    }

    pub fn merge_processor_metadata(&mut self, processor_name: &str, processor_metadata: &ProcessorMetadata) {
        for (key, value) in &processor_metadata.metadata {
            self.add_metadata(processor_name, key, value);
        }
    }

    pub fn merge_processor_response(&mut self, processor_name: &str, response: &ProcessorResponse) {
        if let Some(response_metadata) = &response.metadata {
            // Merge all processor metadata from the response
            for (proc_name, proc_metadata) in &response_metadata.metadata {
                self.merge_processor_metadata(proc_name, proc_metadata);
            }
        }
    }

    pub fn get_processor_metadata(&self, processor_name: &str) -> Option<&ProcessorMetadata> {
        self.metadata.get(processor_name)
    }

    pub fn get_metadata_value(&self, processor_name: &str, key: &str) -> Option<&str> {
        self.metadata.get(processor_name)?.metadata.get(key).map(|s| s.as_str())
    }

    pub fn list_processors(&self) -> Vec<&str> {
        self.metadata.keys().map(|s| s.as_str()).collect()
    }
}
