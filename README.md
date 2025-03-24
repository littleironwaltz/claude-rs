# claude-rs

[![Crates.io](https://img.shields.io/crates/v/claude-rs.svg)](https://crates.io/crates/claude-rs)
[![Docs.rs](https://docs.rs/claude-rs/badge.svg)](https://docs.rs/claude-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A modern, innovative Rust SDK for Anthropic's Claude API.

## Project Structure

The SDK is organized into modular components for better maintainability and separation of concerns:

- **Core Components**
  - `client.rs`: Main Claude client implementation with type-safe futures
  - `types.rs`: Core types, enhanced error handling, and lifetime-parameterized futures
  - `builder.rs`: MessageBuilder for constructing requests
  - `middleware.rs`: Request and response middleware traits
  - `context.rs`: Context management for optimizing token usage
  
- **Domain-Specific Clients**
  - `domains/mod.rs`: Domain client registry with DashMap for lock-free concurrent access
  - `domains/base.rs`: Base domain client implementation
  - `domains/sentiment.rs`: Sentiment analysis capabilities
  - `domains/entity.rs`: Entity extraction functionality
  - `domains/content.rs`: Content generation with templates
  - `domains/code.rs`: Code-focused assistance features
  
- **Utilities**
  - `utils/json_extractor.rs`: JSON extraction utilities
  - `utils/token_counter.rs`: Accurate token counting utilities
  - `reactive.rs`: Reactive extensions for streaming (feature-gated)

## Features

- **Full API Support**: Complete support for Anthropic's Claude API, including messages, streaming, and function calling
- **Domain-Specific APIs**: Type-safe, specialized interfaces for common tasks like sentiment analysis, entity extraction, content generation, and code assistance
- **Optimized Domain Registry**: Lock-free concurrent domain client registry using DashMap with OnceLock caching (4.5x faster)
- **Enhanced Error Handling**: Structured error types with location tracking, source error chaining, and helper methods for better debugging
- **Global Max Tokens Configuration**: Set default token limits at the client level with prioritized override capability
- **Lifetime-Parameterized Futures**: Type-safe futures with proper lifetime parameters
- **Adaptive Context Management**: Smart handling of conversation context to optimize token usage
- **Improved Streaming API**: Support for both new and legacy streaming formats with format-agnostic helper methods
- **Reactive Streaming**: Enhanced streaming capabilities with status tracking and error reporting
- **Template System**: Reusable prompt templates with parameter validation
- **Type Safety**: Comprehensive type system for all API interactions
- **Middleware Support**: Extensible request and response processing pipeline
- **Async/Await**: Built on Tokio for asynchronous operation
- **Simplified Testing**: DomainTester<T> generic pattern for consistent, thread-safe testing
- **Token Optimization Strategies**: Documented patterns for efficient token usage

## Quick Start

Add `claude-rs` to your `Cargo.toml`:

```toml
[dependencies]
claude-rs = "0.2.0"
tokio = { version = "1.0", features = ["full"] }
```

Basic usage:

```rust
// Import common types from the prelude
use claude_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client from ANTHROPIC_API_KEY environment variable
    // With global default max_tokens setting
    let claude = from_env()?
        .with_default_max_tokens(1000)?; // Sets default for all operations
    
    // Send a message with parameter validation
    let response = claude.message()
        .user_message("What's the capital of France?")?
        .temperature(0.7)? // Validated: 0.0 <= temp <= 1.0
        // No need to set max_tokens here, uses client default (1000)
        .send()
        .await?;
        
    // Extract and print the response
    if let Some(Content::Text { text }) = response.content.first() {
        println!("Claude's response: {}", text);
    }
    
    Ok(())
}
```

## Domain-Specific APIs

### Sentiment Analysis

```rust
use claude_rs::domains::sentiment;

// Uses the client's default max_tokens if set, otherwise domain-specific default (500)
let sentiment = claude.domains().sentiment()
    .analyze_text("I absolutely love this product! The quality is excellent.")
    .await?;
    
println!("Sentiment: {:?}, Score: {}", sentiment.sentiment, sentiment.score);

// Analyze specific aspects
let detailed = claude.domains().sentiment()
    .with_aspects("The restaurant had amazing food but poor service.", 
        vec!["food", "service", "price"])
    .await?;

// Override client's default max_tokens for this specific operation
let sentiment = claude.domains().sentiment()
    .analyze_text_with_tokens(
        "I absolutely love this product! The quality is excellent.",
        Some(300) // Uses 300 tokens instead of client default
    ).await?;
```

### Translation

```rust
use claude_rs::domains::translation::TranslationResult;

// Basic translation (uses client default max_tokens if set, otherwise domain default: 1000)
let result = claude.translation()
    .translate("Hello, world!", "Spanish", None::<String>)
    .await?;
    
println!("Translated text: {}", result.translated_text);
// Output: Translated text: Hola, mundo!

// Translation with explicit max_tokens specification (overrides client default)
let result = claude.translation()
    .translate_with_tokens(
        "Hello, world!", 
        "Spanish", 
        None::<String>, 
        Some(1000)
    ).await?;

// Translation with source language specified
let result = claude.translation()
    .translate("Bonjour le monde!", "Japanese", Some("French"))
    .await?;
    
// Language detection (uses lower token count: 500 by default)
let detected = claude.translation()
    .detect_language("こんにちは世界、元気ですか？")
    .await?;
    
println!("Detected language: {} ({}), Confidence: {:.2}", 
        detected.name.unwrap_or_else(|| "Unknown".to_string()),
        detected.language,
        detected.confidence);
        
// Translation with alternative phrasings for idioms (uses higher token count: 1500)
let result = claude.translation()
    .translate_with_alternatives(
        "It's raining cats and dogs, but every cloud has a silver lining.",
        "German", 
        Some(2)
    ).await?;
    
// Access alternative translations
if let Some(alternatives) = result.alternatives {
    for alt in alternatives {
        println!("Original: '{}', Alternative: '{}'", alt.original, alt.alternative);
    }
}

// Advanced usage with explicit token counts
let detected = claude.translation()
    .detect_language_with_tokens(text, Some(500))
    .await?;
    
let result = claude.translation()
    .translate_with_alternatives_and_tokens(
        text, 
        "French", 
        Some(3), 
        Some(2000) // More tokens for more alternatives
    ).await?;
```

### Entity Extraction

```rust
use claude_rs::domains::entity::{self, EntityType};

let entities = claude.domains().entity()
    .extract_from_text("Apple Inc. is planning to open a new store in Tokyo, Japan next year.")
    .await?;
    
for entity in entities {
    println!("Found entity: {} ({})", entity.text, entity.entity_type);
}

// Extract specific types
let locations = claude.domains().entity()
    .with_types("Visit New York and Los Angeles next summer.", 
        vec![EntityType::Location])
    .await?;
```

### Content Generation

```rust
use claude_rs::domains::content::ContentTemplate;

// Using templates with validation
let template = ContentTemplate::new("Write a {{length}} product description for a {{product}}.")?
    .with_param("length", "short")?                // Validation: parameter name must exist
    .with_param("product", "wireless earbuds")?;   // Validation: value cannot be empty
    
// Direct access with the new, cleaner syntax
let product_desc = claude.content()
    .generate_from_template(template)
    .await?;
    
// Specialized content generation with parameter validation
let blog_post = claude.content()
    .blog_post(
        "Artificial Intelligence in Healthcare", // Validated: topic cannot be empty
        Some("professional".to_string()),        // Validated: tone cannot be empty if provided
        Some(800)                               // Validated: 100 <= word_count <= 5000
    )
    .await?;
```

### Code Assistance

```rust
use claude_rs::domains::code;

let code_to_analyze = r#"
fn calculate_total(prices: &[f64], quantities: &[i32]) -> f64 {
    let mut total = 0.0;
    for i in 0..prices.len() {
        total += prices[i] * quantities[i] as f64;
    }
    return total;
}
"#;

let analysis = claude.domains().code()
    .analyze_code(code_to_analyze, "rust")
    .await?;
    
println!("Code analysis: {}", analysis.summary);

// Generate documentation
let docs = claude.domains().code()
    .generate_docs(code_to_analyze, "rust", Some("rustdoc".to_string()))
    .await?;
```

## Streaming

The library now supports both the new and legacy Claude API streaming formats with helper methods for easier handling:

```rust
let stream = claude.message()
    .user_message("Write a short story about a robot learning to paint.")?
    .max_tokens(1000)? // Explicitly set for this request, or it would use client default
    .stream()
    .await?;
    
use futures::StreamExt;
tokio::pin!(stream);

while let Some(result) = stream.next().await {
    match result {
        Ok(delta) => {
            // Use the helper method to extract text from any format
            if let Some(text) = delta.to_text() {
                print!("{}", text);
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
            }
            
            // Check if this is the final event
            if delta.is_final() {
                println!("\n\nGeneration complete!");
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

### Reactive Streaming

For enhanced streaming with status tracking, use the reactive extensions:

```rust
// Create a reactive response
let reactive = claude.send_reactive(
    claude.message()
        .user_message("Explain quantum computing in simple terms.")?
        // Uses client default max_tokens if set
).await?;

// Transform to text stream for easier processing
let mut text_stream = reactive.text_stream();

// Process text chunks as they arrive
while let Some(Ok(chunk)) = text_stream.next().await {
    print!("{}", chunk);
}

// Check status and get complete text
if reactive.is_complete() {
    println!("\n\nFinal text: {}", reactive.current_text());
}
```

For detailed documentation on the streaming API, see [STREAMING.md](STREAMING.md).

## Advanced Features

### Global Token Management

```rust
// Set a global default max_tokens value for all operations
let claude = Claude::new(api_key)
    .with_default_max_tokens(1200)?; // Validated: must be > 0

// The default is applied to all operations that don't specify their own max_tokens
let response = claude.message()
    .user_content("Tell me about quantum physics")? // Uses 1200 tokens by default
    .send()
    .await?;

// Domain clients also use the client default if set
let entities = claude.entity()
    .extract_from_text(text) // Uses 1200 tokens instead of domain default (500)
    .await?;

// Individual operations can override the client default
let response = claude.message()
    .user_content("Explain natural language processing")?
    .max_tokens(2000)? // Overrides the client default (1200)
    .send()
    .await?;

// Domain clients have _with_tokens variants to override the default
let sentiment = claude.sentiment()
    .analyze_text_with_tokens("Great product!", Some(400))
    .await?;
```

### Token Priority Resolution

The system follows a clear priority order for determining max_tokens:
1. Method parameter (if provided)
2. Client default_max_tokens (if set)
3. Domain-specific fallback:
   - Simple operations: 500 tokens (entity, sentiment)
   - Standard operations: 1000 tokens (translation, code)
   - Complex operations: 1500 tokens (content generation)

### Validation & Error Handling

```rust
// Parameter validation with detailed error messages
match claude.message().temperature(1.5) {
    Ok(_) => println!("This shouldn't happen"),
    Err(e) => println!("Error: {}", e), // "Error: Validation error: temperature must be between 0.0 and 1.0, but got 1.5"
}

// Template validation
let template = match ContentTemplate::new("Hello {{name}}!") {
    Ok(t) => t.with_param("name", "World")?,
    Err(e) => panic!("Invalid template: {}", e),
};

// Range validation for numeric parameters
let blog = claude.content()
    .blog_post(
        "AI Safety",
        None,
        Some(100) // Minimum word count validation: must be >= 100
    )
    .await?;

// Validation with location tracking and error chaining
if let Err(e) = claude.with_default_max_tokens(0) {
    println!("Error: {}", e);
    if let Some(location) = e.location() {
        println!("Error occurred at: {}", location);
    }
}
```

### Context Management

```rust
use claude_rs::context::{AdaptiveContextManager, SimpleImportanceScorer};

// Create a client with adaptive context management
let claude = Claude::new(api_key)
    .with_context_manager(AdaptiveContextManager::new(
        4000, // max tokens
        SimpleImportanceScorer
    ));
```

### Function Calling

```rust
use claude_rs::types::Tool;
use serde_json::json;

// Define a tool
let weather_tool = Tool {
    name: "get_weather".to_string(),
    description: "Get the current weather for a location".to_string(),
    input_schema: json!({
        "type": "object",
        "properties": {
            "location": {
                "type": "string",
                "description": "The city and state/country"
            },
            "unit": {
                "type": "string",
                "enum": ["celsius", "fahrenheit"]
            }
        },
        "required": ["location"]
    }),
};

// Ask Claude to use the tool
let response = claude.message()
    .user_message("What's the weather like in Tokyo?")
    .add_tool(weather_tool)
    .max_tokens(1000)? // Override client default if needed
    .send()
    .await?;

// Process tool use request
// ...
```

## Dynamic Domain Registration

The SDK supports high-performance, lock-free dynamic registration of domain clients at runtime using DashMap, which is useful for plugin-based architectures and concurrent applications:

```rust
// Create a custom domain client
struct CustomDomainClient {
    base: BaseDomainClient,
}

impl CustomDomainClient {
    pub fn new(claude: Arc<Claude>) -> Self {
        Self { base: BaseDomainClient::new(claude, "custom_domain") }
    }
    
    pub async fn specialized_operation(&self, input: &str) -> ClaudeResult<String> {
        let prompt = format!("Custom domain processing: {}", input);
        self.text_operation(&prompt, None, self.domain_name()).await
    }
}

// Implement required traits
impl DomainClient for CustomDomainClient {
    fn domain_name(&self) -> &str {
        self.base.domain_name()
    }
}

impl ValidationOperations for CustomDomainClient {}

impl DomainOperations for CustomDomainClient {
    // Implementation details omitted for brevity
}

// Register the custom domain client at runtime
let custom_client = CustomDomainClient::new(Arc::new(claude.clone()));
claude.register_domain("custom_domain", custom_client).await;

// Retrieve and use the registered domain client
if let Some(domain) = claude.get_domain("custom_domain").await {
    // Use the domain client
    // Note: You'll need to downcast it to use specialized methods
}
```

## Testing with Mock API

The SDK provides a comprehensive mock testing infrastructure for testing without making actual API calls:

```rust
use claude_rs::test_helpers::{test_sentiment, create_sentiment_response};

#[tokio::test]
async fn test_sentiment_analysis() {
    // Get a pre-configured domain tester
    let tester = test_sentiment();
    
    // Mock a specific response
    tester.mock_response(
        ClaudeModel::Sonnet,
        create_sentiment_response("Positive", 0.95)
    );
    
    // Test the domain method
    let result = tester.domain_client
        .analyze_text("Great product!")
        .await
        .unwrap();
    
    // Verify the result
    assert_eq!(result.sentiment, Sentiment::Positive);
    assert!(result.score > 0.9);
    
    // Verify the request content
    assert!(tester.assert_request_contains("sentiment"));
    assert!(tester.assert_request_contains("Great product"));
}
```

For detailed documentation on the mock testing infrastructure, see [MOCK_TESTING.md](MOCK_TESTING.md).

## Example Verification

All examples in the `/examples` directory have been verified to work correctly with the current implementation. Here's a summary of the included examples:

- **basic.rs**: Demonstrates basic Claude API usage with error handling
- **streaming.rs**: Shows both standard API and streaming responses with format-agnostic helpers
- **domain_specific.rs**: Showcases domain-specific clients for different tasks
- **translation_example.rs**: Demonstrates translation, language detection, and handling idiomatic expressions
- **function_calling.rs**: Illustrates function calling concepts
- **error_handling.rs**: Demonstrates advanced error handling with location tracking and source chaining
- **validation_examples.rs**: Shows parameter validation across different components
- **concurrent_domain_registry.rs**: Demonstrates the lock-free domain registry with DashMap performance benchmarking
- **testing_pattern.rs**: Non-executable example showing testing patterns with DomainTester<T>
- **max_tokens_example.rs**: Demonstrates global default max_tokens configuration and override patterns

To run any example:

```bash
cargo run --example basic
```

## Documentation

For more detailed documentation on specific topics:

- [CLAUDE.md](CLAUDE.md): Development guidelines and project structure
- [STREAMING.md](STREAMING.md): Detailed documentation on streaming capabilities
- [MOCK_TESTING.md](MOCK_TESTING.md): Guide to using the mock testing infrastructure
- [IMPROVEMENTS.md](IMPROVEMENTS.md): Recent improvements and enhancement details

## License

This project is licensed under the MIT License - see the LICENSE file for details.