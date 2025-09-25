# Rust Language Features: Local Processor Implementation Patterns

This directory showcases Rust's type system, serialization, and data processing patterns for building robust, configurable processors.

## Beginner: Struct-Based Configuration

### Configuration Structs with Serde
```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChangeTextCaseConfig {
    pub case_type: CaseType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaseType {
    Upper,
    Lower,
    Proper,
    Title,
    #[serde(untagged)]
    Custom(String),
}
```

**Key Rust features:**
- **Derive macros**: Auto-generate common trait implementations
- **Serde attributes**: Control serialization behavior
- **`#[serde(rename_all = "lowercase")]`**: Maps `CaseType::Upper` to `"upper"` in YAML
- **`#[serde(untagged)]`**: Fallback variant for unknown values

### Constructor Patterns
```rust
impl ChangeTextCaseProcessor {
    pub fn new(config: ChangeTextCaseConfig) -> Self {
        Self { config }
    }

    // Convenience constructors for common cases
    pub fn upper() -> Self {
        Self::new(ChangeTextCaseConfig {
            case_type: CaseType::Upper,
        })
    }
    
    pub fn lower() -> Self {
        Self::new(ChangeTextCaseConfig {
            case_type: CaseType::Lower,
        })
    }
}
```

**Design benefits:**
- **Flexible construction**: Both config-driven and convenience methods
- **Type safety**: Configuration is validated at construction time
- **Immutable by default**: Processors are configured once and don't change

## Intermediate: Pattern Matching and String Processing

### Exhaustive Pattern Matching
```rust
let result = match &self.config.case_type {
    CaseType::Upper => input.to_uppercase(),
    CaseType::Lower => input.to_lowercase(),
    CaseType::Proper => {
        // Complex processing logic
        input.split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
    CaseType::Title => { /* ... */ }
    CaseType::Custom(custom_type) => {
        return ProcessorResponse {
            outcome: Some(Outcome::Error(ErrorDetail {
                code: 400,
                message: format!("Unsupported custom case type: {}", custom_type),
            })),
        };
    }
};
```

**Advanced Rust patterns:**
- **Exhaustive matching**: Compiler ensures all enum variants are handled
- **Reference patterns**: `&self.config.case_type` avoids moving the enum
- **Iterator chains**: `split_whitespace().map().collect()` for functional processing
- **Early returns**: Handle error cases immediately

### String Processing Techniques
```rust
// Proper case implementation
input.split_whitespace()
    .map(|word| {
        let mut chars = word.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => {
                first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
            }
        }
    })
    .collect::<Vec<_>>()
    .join(" ")
```

**Rust string handling:**
- **`chars()`**: Unicode-aware character iteration
- **`collect::<String>()`**: Collect iterator into String
- **`as_str()`**: Convert `Chars` iterator back to string slice
- **Functional style**: Chain operations without intermediate variables

## Advanced: Serialization and Data Transformation

### JSON Processing with Serde
```rust
// In TokenCounterProcessor
let stats = TokenStats {
    char_count: input.chars().count(),
    word_count: input.split_whitespace().count(),
    line_count: input.lines().count(),
};

let json_output = serde_json::to_string(&stats)
    .map_err(|e| format!("Failed to serialize token stats: {}", e))?;

ProcessorResponse {
    outcome: Some(Outcome::NextPayload(json_output.into_bytes())),
}
```

**Serialization patterns:**
- **Structured output**: Use structs instead of ad-hoc JSON
- **Error handling**: Convert serde errors to processor errors
- **Type safety**: Serde ensures consistent JSON structure

### HashMap-Based Data Processing
```rust
// In WordFrequencyAnalyzerProcessor
let mut word_counts: HashMap<String, usize> = HashMap::new();

for word in input.split_whitespace() {
    let normalized = word.to_lowercase();
    *word_counts.entry(normalized).or_insert(0) += 1;
}

let frequency_map = WordFrequency { word_counts };
```

**HashMap techniques:**
- **`entry().or_insert()`**: Insert if not present, otherwise get existing value
- **Mutable references**: `*word_counts.entry()` dereferences to modify value
- **Normalization**: Convert to lowercase for case-insensitive counting

### Configuration-Driven Behavior
```rust
// In PrefixSuffixAdderProcessor
pub fn with_prefix_and_suffix(prefix: String, suffix: String) -> Self {
    Self {
        config: PrefixSuffixConfig { prefix, suffix }
    }
}

async fn process(&self, req: ProcessorRequest) -> ProcessorResponse {
    let input = String::from_utf8(req.payload)?;
    let result = format!("{}{}{}", self.config.prefix, input, self.config.suffix);
    
    ProcessorResponse {
        outcome: Some(Outcome::NextPayload(result.into_bytes())),
    }
}
```

## Key Rust Concepts Demonstrated

### 1. **Owned vs Borrowed Strings**
```rust
// String (owned) vs &str (borrowed)
pub struct Config {
    pub prefix: String,  // Owned - processor owns this data
}

fn process_text(input: &str) -> String {  // Borrowed input, owned output
    input.to_uppercase()
}
```

### 2. **Error Handling in Processors**
```rust
let input = match String::from_utf8(req.payload) {
    Ok(text) => text,
    Err(e) => {
        return ProcessorResponse {
            outcome: Some(Outcome::Error(ErrorDetail {
                code: 400,
                message: format!("Invalid UTF-8 input: {}", e),
            })),
        };
    }
};
```

**Pattern benefits:**
- **Early error detection**: Validate input immediately
- **Structured errors**: Use ErrorDetail instead of panicking
- **Graceful degradation**: Return error response instead of crashing

### 3. **Iterator Combinators**
```rust
// Functional programming style
let words: Vec<&str> = input
    .split_whitespace()
    .filter(|word| !word.is_empty())
    .collect();

// Equivalent imperative style
let mut words = Vec::new();
for word in input.split_whitespace() {
    if !word.is_empty() {
        words.push(word);
    }
}
```

### 4. **Type-Driven Development**
```rust
#[derive(Serialize)]
struct TokenStats {
    char_count: usize,
    word_count: usize,
    line_count: usize,
}

#[derive(Serialize)]
struct WordFrequency {
    word_counts: HashMap<String, usize>,
}
```

**Benefits:**
- **Self-documenting**: Structure shows exactly what data is returned
- **Compile-time validation**: Serde ensures serialization correctness
- **API stability**: Changes to structure are breaking changes (good!)

## Design Patterns Applied

### 1. **Builder Pattern Variant**
```rust
impl ChangeTextCaseProcessor {
    pub fn upper() -> Self { /* ... */ }
    pub fn lower() -> Self { /* ... */ }
    pub fn proper() -> Self { /* ... */ }
    pub fn title() -> Self { /* ... */ }
}
```

### 2. **Strategy Pattern with Enums**
```rust
enum CaseType {
    Upper,    // Strategy: to_uppercase()
    Lower,    // Strategy: to_lowercase()
    Proper,   // Strategy: custom proper case logic
    Title,    // Strategy: title case with exceptions
}
```

### 3. **Template Method Pattern**
```rust
// All processors follow the same template:
async fn process(&self, req: ProcessorRequest) -> ProcessorResponse {
    // 1. Parse input
    let input = String::from_utf8(req.payload)?;
    
    // 2. Process (varies by processor)
    let result = self.process_text(&input);
    
    // 3. Return response
    ProcessorResponse {
        outcome: Some(Outcome::NextPayload(result.into_bytes())),
    }
}
```

## Performance Considerations

### 1. **String Allocation Awareness**
```rust
// ❌ Inefficient: Multiple allocations
let result = input.to_uppercase() + &suffix;

// ✅ Better: Pre-allocate capacity
let mut result = String::with_capacity(input.len() + suffix.len());
result.push_str(&input.to_uppercase());
result.push_str(&suffix);
```

### 2. **Iterator Efficiency**
```rust
// ✅ Lazy evaluation - no intermediate collections
input.split_whitespace()
    .map(|word| word.to_lowercase())
    .collect::<Vec<_>>()
```

### 3. **Avoid Unnecessary Cloning**
```rust
// ✅ Work with references when possible
fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
}
```

This local processor implementation demonstrates how Rust's type system, pattern matching, and functional programming features combine to create robust, efficient, and maintainable text processing components.
