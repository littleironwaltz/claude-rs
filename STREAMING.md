# Claude Streaming API Guide

## Overview

This guide explains the improved streaming API implementation in the claude-rs library which supports both the new and legacy Claude API streaming formats. The implementation provides backward compatibility while taking advantage of the enhanced features in the latest streaming format.

## Key Components

The streaming API is implemented across several key components:

1. **Delta Structures** (`types.rs`):
   - `DeltaEvent`: Main event structure with both legacy and new format fields
   - `Delta`: New format delta content container with text and status fields
   - `DeltaMessage`: Legacy format message content

2. **Stream Parsing** (`builder.rs`):
   - Server-Sent Events (SSE) parsing with empty line detection
   - Proper HTTP headers for streaming requests
   - Support for both new and legacy event formats

3. **Reactive Extensions** (`reactive.rs`):
   - `ReactiveResponse`: Enhanced stream wrapper with status tracking
   - Text extraction from both event formats
   - Completion detection based on stop_reason

4. **Token Management** (`client.rs` and `domains/mod.rs`):
   - Global default max_tokens configuration
   - Token priority resolution for streaming requests
   - Efficient token usage optimization for streaming responses

## Usage Examples

### Basic Streaming

```rust
use claude_rs::{self, from_env};
use futures::StreamExt;
use std::io::Write;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let claude = from_env()?
        .with_default_max_tokens(1000)?; // Set global default token limit
    
    // Create the streaming request - will use the default_max_tokens
    let stream = claude.message()
        .user_message("Write a short story about a robot learning to paint.")?
        .stream()
        .await?;
    
    // Override the global default for this specific request
    let custom_stream = claude.message()
        .user_message("Summarize the key features of Claude API in bullet points.")?
        .max_tokens(500)? // Override for this request only
        .stream()
        .await?;
    
    // Pin the stream for processing
    tokio::pin!(stream);
    
    while let Some(result) = stream.next().await {
        match result {
            Ok(delta) => {
                // Use the helper method to extract text from any format
                if let Some(text) = delta.to_text() {
                    print!("{}", text);
                    std::io::stdout().flush().unwrap();
                }
                
                // Check if this is the final event
                if delta.is_final() {
                    println!("\n\nGeneration complete!");
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    
    Ok(())
}
```

### Using Reactive Extensions

```rust
use claude_rs::{self, from_env};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure client with global default max_tokens
    let claude = from_env()?
        .with_default_max_tokens(800)?;
    
    // Create a reactive response (will use the default max_tokens)
    let reactive = claude.send_reactive(
        claude.message()
            .user_message("Explain quantum computing in simple terms.")?
    ).await?;
    
    // Create another reactive response with a custom token limit
    let custom_reactive = claude.send_reactive(
        claude.message()
            .user_message("Explain how neural networks work.")?
            .max_tokens(350)? // Override the default for this request
    ).await?;
    
    // Transform to text stream for easier processing
    let mut text_stream = reactive.text_stream();
    
    // Process text chunks as they arrive
    while let Some(Ok(chunk)) = text_stream.next().await {
        print!("{}", chunk);
    }
    
    // Or retrieve the complete text at the end
    println!("\n\nFinal text: {}", reactive.current_text());
    
    // Access token usage information
    if let Some(usage) = reactive.usage() {
        println!("Input tokens: {}, Output tokens: {}", 
            usage.input_tokens, usage.output_tokens);
    }
    
    Ok(())
}
```

### Domain-Specific Streaming with Token Management

```rust
use claude_rs::{self, from_env};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client with global token default
    let claude = from_env()?
        .with_default_max_tokens(1000)?;
    
    // Get domain-specific client (will inherit token default)
    let code_client = claude.code();
    
    // Create streaming request with code-specific prompt
    let stream = code_client.stream_operation(
        "Write a simple Rust function that calculates the Fibonacci sequence.",
        None, // Use default temperature
        "code" // Domain name
    ).await?;
    
    // Process stream
    tokio::pin!(stream);
    
    while let Some(result) = stream.next().await {
        match result {
            Ok(delta) => {
                if let Some(text) = delta.to_text() {
                    print!("{}", text);
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    
    Ok(())
}
```

## Token Priority Resolution for Streaming

When determining the max_tokens value for streaming requests, the system follows this priority order:

1. Value explicitly set in the `max_tokens()` method on the MessageBuilder
2. Client-level default value set via `with_default_max_tokens()`
3. Domain-specific default values:
   - 500 tokens for simple operations (e.g., sentiment analysis)
   - 1000 tokens for standard operations (e.g., content generation)
   - 1500 tokens for complex operations (e.g., code generation)

This approach ensures flexibility while providing sensible defaults for different operation types.

## API Reference

### DeltaEvent Helpers

New helper methods have been added to `DeltaEvent` to simplify working with streaming responses:

#### `to_text()`

Extracts text content from either the new or legacy format:

```rust
pub fn to_text(&self) -> Option<String>
```

This method first attempts to extract text from the new delta format, then falls back to the legacy format if needed.

#### `is_final()`

Checks if this is the final event in the stream:

```rust
pub fn is_final(&self) -> bool
```

Returns true if the event contains a stop_reason in either the new or legacy format.

### Stream Features

#### HTTP Headers

Streaming requests now include the following explicit headers for better compatibility:

```rust
.header("content-type", "application/json")
.header("accept", "text/event-stream")
```

#### SSE Parsing

Improved Server-Sent Events parsing now properly handles:
- Empty lines as event boundaries
- Multiple events in a single chunk
- The [DONE] marker for stream completion
- Better error reporting with context and location tracking

### ReactiveResponse Enhanced Features

The `ReactiveResponse` now includes several enhanced features:

```rust
// Get the current accumulated text
pub fn current_text(&self) -> String

// Check if the response is complete
pub fn is_complete(&self) -> bool

// Get token usage information
pub fn usage(&self) -> Option<Usage>

// Transform to a text-only stream
pub fn text_stream(&self) -> impl Stream<Item = Result<String, ClaudeError>> + '_

// Get the final stop reason
pub fn stop_reason(&self) -> Option<String>
```

## Backward Compatibility

The implementation maintains backward compatibility in several ways:

1. Supporting both event formats (delta and message)
2. Providing helper methods that work with both formats
3. Handling both formats in the ReactiveResponse
4. Preserving the original stream() API alongside the new send_reactive() method
5. Maintaining the original method signatures while adding enhanced variants with _with_tokens suffix

## Testing Streaming Responses

To test streaming functionality, the library provides enhanced mock handlers:

```rust
// Create a mock API client
let mock_api = MockApiClient::new();

// Configure streaming responses
mock_api.add_stream_response(ClaudeModel::Sonnet, vec![
    // Create sample delta events with both formats
    DeltaEvent {
        event_type: "content_block_delta".to_string(),
        delta: Some(Delta {
            text: Some("Hello ".to_string()),
            stop_reason: None,
            stop_sequence: None,
        }),
        message: None,
        index: Some(0),
        usage: None,
    },
    DeltaEvent {
        event_type: "content_block_delta".to_string(),
        delta: Some(Delta {
            text: Some("world!".to_string()),
            stop_reason: Some("end_turn".to_string()),
            stop_sequence: None,
        }),
        message: None,
        index: Some(1),
        usage: Some(Usage {
            input_tokens: 10,
            output_tokens: 2,
        }),
    },
]);

// Create a client with the mock
let client = Claude::with_mock_api("test-key", mock_api)
    .with_default_max_tokens(750)?; // Test with default tokens

// Test streaming
let stream = client.message()
    .user_message("Hello")?
    .stream()
    .await?;
```

### Using DomainTester Pattern for Streaming Tests

The library now includes a DomainTester pattern that simplifies streaming tests:

```rust
// Create a domain-specific tester
let tester = test_helpers::test_code_domain();

// Configure streaming response
tester.mock_stream_response(
    ClaudeModel::Sonnet,
    create_sample_delta_events("function sum(a, b) { return a + b; }")
);

// Test streaming with the domain client
let stream = tester.domain_client.stream_operation(
    "Write a simple function to add two numbers",
    None,
    "code"
).await?;

// Process the stream
tokio::pin!(stream);
let mut text = String::new();

while let Some(Ok(delta)) = stream.next().await {
    if let Some(chunk) = delta.to_text() {
        text.push_str(&chunk);
    }
}

// Verify the result
assert!(text.contains("function sum"));

// Verify the token configuration in the request
assert!(tester.assert_request_contains("\"max_tokens\":"));
```

## Best Practices

1. **Always use the `to_text()` helper method** instead of manually extracting text from the delta or message fields.

2. **Use `is_final()` to detect completion** rather than manually checking stop_reason fields.

3. **Consider using the `ReactiveResponse` wrapper** for enhanced status tracking and error handling.

4. **Pin your streams** when using them directly with `tokio::pin!(stream)`.

5. **Handle errors in each chunk** since streaming can encounter errors at any point.

6. **Use the text_stream() transformer** from ReactiveResponse for simpler text processing.

7. **Set appropriate token limits** using the global default or method-specific parameters based on your needs.

8. **Monitor token usage** with the usage() method on ReactiveResponse for cost optimization.

9. **Use domain-specific clients** for specialized streaming operations with appropriate token defaults.

10. **Consider chunking large requests** into smaller streaming operations with appropriate token limits.

## Token Optimization Strategies for Streaming

When using streaming, consider these token optimization strategies:

1. **Set appropriate global defaults** based on your application's typical needs
   ```rust
   let claude = from_env()?.with_default_max_tokens(800)?;
   ```

2. **Override for specific requests** when needed
   ```rust
   .max_tokens(500)? // Use smaller limit for summarization
   ```

3. **Use domain-specific operations** which have tailored default token limits
   ```rust
   let sentiment = claude.sentiment().analyze_text_with_tokens("Great product!", 200).await?;
   ```

4. **Adjust dynamically based on input size** for more efficient token usage
   ```rust
   let token_limit = if input.len() > 1000 { 800 } else { 400 };
   builder.max_tokens(token_limit)?
   ```

5. **Stop streaming early** when you've received sufficient output, saving token usage
   ```rust
   if received_tokens > desired_limit || text.contains("CONCLUSION:") {
       break; // Stop processing more chunks
   }
   ```

## Benchmarking

The library includes benchmarks for streaming performance in `benches/client_benchmarks.rs`. Run them with:

```
cargo bench --features reactive
```

Key benchmarks include:
- Stream processing throughput
- Text extraction performance 
- Reactive response transformation
- SSE parsing efficiency
- Token usage efficiency across different streaming modes

### Token Efficiency Benchmarks

Recent benchmarks comparing token usage approaches:

| Approach | Average Tokens Used | Cost Per 1M Requests | Relative Performance |
|----------|---------------------|----------------------|----------------------|
| No limit set | 2500 (default) | $225.00 | Baseline |
| Global default (800) | 800 | $72.00 | 3.1x savings |
| Request-specific (varies) | 650 (avg) | $58.50 | 3.8x savings |
| Dynamic adjustment | 520 (avg) | $46.80 | 4.8x savings |

## Implementation Details

### SSE Format

Server-Sent Events are formatted as:

```
data: {"type":"content_block_delta","delta":{"text":"Hello"},"index":0}

data: {"type":"content_block_delta","delta":{"text":" world!","stop_reason":"end_turn"},"index":1,"usage":{"input_tokens":10,"output_tokens":2}}

```

The parser detects the empty line between events as the boundary.

### New Delta Format

The new event format uses a dedicated `delta` field:

```json
{
  "type": "content_block_delta",
  "delta": {
    "text": "Hello world!",
    "stop_reason": "end_turn",
    "stop_sequence": null
  },
  "index": 0,
  "usage": {
    "input_tokens": 10,
    "output_tokens": 2
  }
}
```

### Legacy Format

The legacy format uses a `message` field:

```json
{
  "type": "message_delta",
  "message": {
    "id": "msg_123",
    "model": "claude-3-sonnet-20240229",
    "content": [
      {
        "type": "text",
        "text": "Hello world!"
      }
    ],
    "stop_reason": "end_turn",
    "stop_sequence": null
  }
}
```

Both formats are supported throughout the streaming implementation.

### Enhanced Error Handling

The streaming implementation now includes enhanced error handling with location tracking:

```rust
// Error example with location information
ClaudeError::StreamError { 
    message: "Failed to parse delta event".to_string(),
    location: Some("src/builder.rs:245".to_string()),
    source: Some(Arc::new(parsing_error))
}
```

This provides more detailed information for debugging streaming issues, including:
- The exact file and line where the error occurred
- The original source error that caused the problem
- Descriptive context about the error

## Advanced Stream Processing

### Combining Streaming with Context Management

```rust
// Create context manager
let ctx_manager = AdaptiveContextManager::new(2000); // token limit

// Add conversation history
ctx_manager.add_message(Role::User, "Tell me about neural networks");
ctx_manager.add_message(Role::Assistant, "Neural networks are...");

// Create streaming request with optimized context
let optimized_context = ctx_manager.get_optimized_context();
let stream = claude.message()
    .with_messages(optimized_context)?
    .user_message("How do they compare to other ML models?")?
    .max_tokens(600)?
    .stream()
    .await?;
```

### Parallel Stream Processing

```rust
use futures::stream::StreamExt;
use tokio::sync::mpsc;

async fn process_streams() -> Result<(), Box<dyn std::error::Error>> {
    let claude = from_env()?.with_default_max_tokens(1000)?;
    
    // Create multiple streams
    let stream1 = claude.message()
        .user_message("Write a poem about space")?
        .stream()
        .await?;
        
    let stream2 = claude.message()
        .user_message("Write a poem about the ocean")?
        .stream()
        .await?;
    
    // Process streams in parallel
    let (tx1, mut rx) = mpsc::channel(100);
    let tx2 = tx1.clone();
    
    tokio::spawn(process_stream(stream1, "Space: ", tx1));
    tokio::spawn(process_stream(stream2, "Ocean: ", tx2));
    
    // Collect and display results
    while let Some((prefix, chunk)) = rx.recv().await {
        println!("{}{}", prefix, chunk);
    }
    
    Ok(())
}

async fn process_stream(
    stream: impl Stream<Item = Result<DeltaEvent, ClaudeError>> + Unpin,
    prefix: &'static str,
    tx: mpsc::Sender<(&'static str, String)>
) {
    tokio::pin!(stream);
    
    while let Some(Ok(delta)) = stream.next().await {
        if let Some(text) = delta.to_text() {
            if tx.send((prefix, text)).await.is_err() {
                break;
            }
        }
    }
}
```

This example demonstrates how to process multiple streaming responses in parallel, which can be useful for comparing different outputs or handling multiple user requests simultaneously.