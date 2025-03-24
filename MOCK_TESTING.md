# Mock Testing Guide for Claude Rust SDK

## Introduction

This guide explains how to use the mock infrastructure to write tests for the Claude Rust SDK. By using mocks, you can test your code without making actual API calls to Anthropic's Claude API service.

## Mock Infrastructure Overview

The Claude Rust SDK provides a robust mock infrastructure that allows you to:

1. Create mock API clients that simulate responses
2. Configure specific responses for different models
3. Record and verify requests made to the API
4. Test both standard and streaming responses
5. Simulate errors and timeouts
6. Use domain-specific test helpers for consistent testing patterns

## JSON Format Requirements for Mock Responses

When creating mock responses for domain-specific clients, the JSON structure must match what the client expects. Here are the required formats for each domain client:

### Sentiment Analysis Client

The JSON response must include:
```json
{
  "sentiment": "Positive" | "Negative" | "Neutral",
  "score": 0.95,  // A number between 0 and 1
  "explanation": "Optional explanation of the sentiment analysis",
  "aspects": {    // Optional detailed analysis by aspect
    "user_experience": {
      "sentiment": "Positive" | "Negative" | "Neutral",
      "score": 0.89,
      "highlights": ["easy to use", "intuitive interface"]
    }
  }
}
```

- The `sentiment` field must exactly match one of the enum variants: "Positive", "Negative", or "Neutral"
- The `score` field must be a number between 0 and 1
- The `aspects` field is optional but follows the same structure

### Entity Extraction Client

The JSON response must include an array of entities:
```json
[
  {
    "text": "Entity text",
    "entity_type": "Person",
    "start_idx": 10,
    "end_idx": 20,
    "confidence": 0.95,
    "metadata": {
      "additional": "information"
    }
  }
]
```

Required fields:
- `text`: The extracted entity text
- `entity_type`: The type of entity (e.g., "Person", "Organization", "Location")
- `start_idx` and `end_idx`: Integer positions in the original text
- `confidence`: A number between 0 and 1
- `metadata`: An object containing additional information (can be empty `{}` or `null`)

### Code Analysis Client

The JSON response must include:
```json
{
  "issues": [
    {
      "message": "Description of the issue",
      "line": 10,
      "column": 5,
      "severity": "Low" | "Medium" | "High",
      "code": "Optional issue code"
    }
  ],
  "suggestions": [
    {
      "description": "Suggestion description",
      "code": "Suggested code",
      "original_code": "Original code to replace",
      "explanation": "Why this suggestion is made"
    }
  ],
  "complexity_score": 3,
  "summary": "Code summary"
}
```

Important notes:
- The `severity` field must exactly match one of the enum variants: "Low", "Medium", or "High"
- The `line` and `column` fields are optional integers

### Content Generation Client

The Content Generation Client accepts varying response formats depending on the method being called. Generally, the response will be plain text without any special JSON structure.

## Best Practices for Mock JSON Responses

1. **Use code blocks**: Wrap JSON responses in markdown code blocks to help the extractor:
   ```
   ```json
   {
     "result": "value"
   }
   ```
   ```

2. **Match expected fields exactly**: Ensure field names and types exactly match what the client expects

3. **Include all required fields**: Missing required fields will cause parsing errors

4. **Test both happy and error paths**: Create responses for both success and error scenarios

5. **Validate your JSON**: Use tools like [JSONLint](https://jsonlint.com/) to validate your JSON before using it in tests

## Using the Mock Infrastructure

### Basic Usage Example

Here's a simple example of how to use the mock infrastructure:

```rust
use std::sync::Arc;
use claude_rs::{Claude, ClaudeModel};
use tests::mock_api_client::{MockApiClient, create_json_response, mock_api_to_handler};

#[tokio::test]
async fn test_basic_example() {
    // Create a mock API client
    let mock_api = Arc::new(MockApiClient::new());
    
    // Configure it with a response
    mock_api.add_response(
        ClaudeModel::Sonnet,
        create_json_response(r#"{"result": "success"}"#),
    );
    
    // Create a Claude client using the mock
    let client = Claude::with_mock_api(
        "test-api-key",
        mock_api_to_handler(mock_api.clone()),
    ).with_model(ClaudeModel::Sonnet);
    
    // Use the client as you normally would
    let result = client.message()
        .user_content("Test message")
        .max_tokens(500)? // Always include max_tokens parameter
        .send()
        .await
        .unwrap();
    
    // Verify the result
    assert!(result.content_text().contains("success"));
    
    // Verify the request that was sent
    let requests = mock_api.get_request_history();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].model, "claude-3-sonnet-20240229");
    assert!(requests[0].max_tokens.is_some(), "max_tokens parameter should be included");
}
```

### Using the DomainTester Pattern (Recommended)

The SDK provides a newer, more consistent way to test domain-specific clients using the `DomainTester` pattern:

```rust
#[tokio::test]
async fn test_sentiment_analysis() {
    // Get a pre-configured domain tester for sentiment analysis
    let tester = test_helpers::test_sentiment();
    
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
    
    // Verify the request content with a single line
    assert!(tester.assert_request_contains("sentiment"));
    assert!(tester.assert_request_contains("Great product"));
}
```

Available test helpers:
- `test_sentiment()`: Creates a tester for sentiment analysis
- `test_entity()`: Creates a tester for entity extraction
- `test_content()`: Creates a tester for content generation
- `test_code()`: Creates a tester for code assistance

### Testing Domain-Specific Clients

To test domain-specific clients, you can use the helper functions:

```rust
async fn test_sentiment_client() {
    // Create JSON response for sentiment analysis
    let sentiment_json = r#"{
        "sentiment": "Positive",
        "score": 0.92
    }"#;
    
    // Setup mock client with response
    let (client, mock_api) = setup_mock_with_json_response(sentiment_json).await;
        
    // Get the sentiment client
    let sentiment_client = client.sentiment();
    
    // Test analyze_text method
    let result = sentiment_client.analyze_text("I love this product!").await.unwrap();
    
    // Verify the result
    assert_eq!(result.sentiment, Sentiment::Positive);
    assert!(result.score > 0.9);
    
    // Verify the request contains correct sentiment analysis text
    assert_request_contains(&mock_api, "sentiment");
    assert_request_contains(&mock_api, "I love this product");
}
```

### Enhanced Error Handling Testing

The mock infrastructure supports advanced error handling testing with location tracking and source chaining:

```rust
#[tokio::test]
async fn test_error_handling() {
    // Create a mock with an error response that includes location information
    let location = concat!(file!(), ":", line!());
    let error = ClaudeError::api_error(
        "Rate limit exceeded", 
        Some(429), 
        None,
        Some(location)
    );
    
    let (client, _) = create_mock_claude_with_error(
        "test-api-key",
        ClaudeModel::Sonnet,
        error
    );
    
    // Get a domain client
    let sentiment_client = client.sentiment();
    
    // Test that errors are properly propagated
    let result = sentiment_client.analyze_text("Test").await;
    assert!(result.is_err());
    
    // Check error details including location tracking
    if let Err(ClaudeError::ApiError { status, location, .. }) = result {
        assert_eq!(status, 429);
        assert!(location.is_some());
        // Verify location was preserved
        assert_eq!(location.unwrap(), location);
    } else {
        panic!("Expected ApiError with status 429");
    }
}
```

### Testing Streaming Responses

If you're using the `reactive` feature, you can test streaming responses:

```rust
#[cfg(feature = "reactive")]
#[tokio::test]
async fn test_streaming() {
    let mock_api = Arc::new(MockApiClient::new());
    
    // Add a streaming response
    mock_api.add_stream_response(
        ClaudeModel::Sonnet, 
        create_test_delta_sequence()
    );
    
    let client = Claude::with_mock_api(
        "test-api-key",
        mock_api_to_handler(mock_api.clone()),
    ).with_model(ClaudeModel::Sonnet);
    
    let builder = client.message().user_content("Test streaming message");
    let reactive = client.send_reactive(builder).await.unwrap();
    
    // Process the streaming response
    let mut text_stream = reactive.text_stream();
    let mut result = String::new();
    
    while let Some(chunk) = text_stream.next().await {
        match chunk {
            Ok(text) => result.push_str(&text),
            Err(e) => panic!("Stream error: {}", e),
        }
    }
    
    // Verify the complete text
    assert_eq!(result, "This is a streaming response.");
}
```

Using the new DomainTester pattern with streaming:

```rust
#[cfg(feature = "reactive")]
#[tokio::test]
async fn test_streaming_with_domain_tester() {
    // Get a pre-configured domain tester
    let tester = test_helpers::test_content();
    
    // Mock a streaming response with text chunks
    tester.mock_stream(
        ClaudeModel::Sonnet,
        vec!["This is ", "a streaming ", "response."]
    );
    
    // Test the streaming operation
    let builder = tester.client.message()
        .user_content("Test streaming message")
        .max_tokens(500)
        .unwrap();
    
    let reactive = tester.client.send_reactive(builder).await.unwrap();
    
    // Process the streaming response
    let mut text_stream = reactive.text_stream();
    let mut result = String::new();
    
    while let Some(chunk) = text_stream.next().await {
        match chunk {
            Ok(text) => result.push_str(&text),
            Err(e) => panic!("Stream error: {}", e),
        }
    }
    
    // Verify the complete text
    assert_eq!(result, "This is a streaming response.");
    
    // Verify the request
    assert!(tester.assert_request_contains("Test streaming message"));
}
```

## Helper Functions

The test suite includes helper functions for working with mock responses:

```rust
// Create a mock API client and Claude client with JSON response
async fn setup_mock_with_json_response(json_response: &str) -> (Arc<Claude>, Arc<MockApiClient>) {
    let mock_api = Arc::new(MockApiClient::new());
    
    // Create a response with the JSON wrapped in a code block to help the extractor
    let mut response = create_sample_message_response();
    response.content = vec![Content::Text { 
        text: format!("```json\n{}\n```", json_response) 
    }];
    
    mock_api.add_response(ClaudeModel::Sonnet, response);
    
    let client = Arc::new(Claude::with_mock_api(
        "test-api-key",
        mock_api_to_handler(mock_api.clone()),
    ).with_model(ClaudeModel::Sonnet));
    
    (client, mock_api)
}

// Verify that a request contains expected content
fn assert_request_contains(mock_api: &MockApiClient, expected_content: &str) {
    let requests = mock_api.get_request_history();
    assert!(!requests.is_empty(), "No requests were made");
    
    let user_messages = requests[0].messages.iter()
        .filter(|msg| msg.role == Role::User)
        .collect::<Vec<_>>();
    
    assert!(!user_messages.is_empty(), "No user messages in the request");
    
    let content_text = match &user_messages[0].content[0] {
        Content::Text { text } => text,
        _ => panic!("Expected text content"),
    };
    
    assert!(content_text.contains(expected_content), 
            "User message does not contain expected content.\nExpected to find: {}\nActual content: {}", 
            expected_content, content_text);
}
```

### Domain-Specific Helper Functions

The mock API client provides domain-specific helper functions for creating responses:

```rust
// Create a sentiment analysis response
let response = create_sentiment_response("Positive", 0.92);

// Create an entity extraction response
let response = create_entity_response(vec![
    ("Apple", "Organization"),
    ("Steve Jobs", "Person"),
    ("California", "Location")
]);

// Create a code analysis response
let response = create_code_analysis_response(vec![
    ("Unused variable", "Medium"),
    ("Potential null reference", "High")
], 3);

// Create a JSON response
let response = create_json_response(r#"{"key": "value"}"#);

// Create a text response
let response = create_text_response("Hello, world!");

// Create a sample response
let response = create_sample_message_response();
```

### Streaming Helper Functions

For testing streaming responses:

```rust
// Create a stream of delta events
let events = create_sample_delta_events();

// Create a custom stream with specific text chunks
let events = create_mock_stream_response(
    vec!["This is ", "a streaming ", "response"],
    true // Include final event
);

// Set up a client with streaming text
let (client, mock_api) = setup_mock_with_streaming_text(
    vec!["This is ", "a streaming ", "response"]
).await;
```

## MockApiClient Configuration

The `MockApiClient` class has several enhanced configuration options:

```rust
// Create a new MockApiClient
let mock_api = Arc::new(MockApiClient::new());

// Add a response (any type that can be converted to MockResponse)
mock_api.add_mock(ClaudeModel::Sonnet, response);

// Legacy methods
mock_api.add_response(ClaudeModel::Sonnet, message_response);
mock_api.add_stream_response(ClaudeModel::Sonnet, delta_events);
mock_api.add_error(ClaudeModel::Sonnet, claude_error);
mock_api.add_stream_error(ClaudeModel::Sonnet, claude_error);

// Add a simulated processing delay
mock_api.with_delay(Duration::from_millis(50));

// Use deterministic timing for stable tests
mock_api.with_deterministic_timing();

// Get request history
let requests = mock_api.get_request_history();

// Clear request history
mock_api.clear_request_history();
```

## Common Issues and Troubleshooting

### JSON Parsing Failures

If you encounter JSON parsing failures:
- Check that field names match exactly what the client expects
- Ensure enum values match expected values (e.g., "Positive" for sentiment)
- Verify that numeric fields are actually numbers, not strings
- Make sure arrays are properly formatted with square brackets
- Wrap the JSON in code blocks for the extractor: ```json {...} ```

### Request Verification Failures

If your request verification fails:
- Use `mock_api.get_request_history()` to inspect the actual requests
- Check the user message content to verify what's being sent
- Use `assert_request_contains()` to verify specific content in requests
- Use `tester.assert_request_contains()` with the DomainTester pattern

### Streaming Response Failures

If you're having issues with streaming responses:
- Ensure the reactive feature flag is enabled
- Check that delta events have the correct format
- Make sure the stream is correctly consumed
- Check for timeouts or early cancellation
- Verify DeltaEvent format matches the API version you're testing

### Error Handling Failures

If you're having issues with error testing:
- Check that error variants match exactly (ApiError vs RequestError)
- Verify that location information is correctly propagated
- Check that error fields match what the test expects
- Use the error helper methods (api_error, request_error) for consistent creation

## Best Practices for Testing

1. **Use the DomainTester pattern**: This provides a consistent, simplified approach for testing domain clients.

2. **Test all domain clients**: Create comprehensive tests for sentiment, entity, content and code clients.

3. **Test both success and error cases**: Make sure your code handles errors gracefully.

4. **Validate requests and responses**: Use request history and assertions to verify correct content.

5. **Use deterministic timing**: For streaming tests, use `with_deterministic_timing()` for stable results.

6. **Mock realistic responses**: Match the format that Claude would actually return.

7. **Use domain-specific response creators**: The helper functions ensure correct formatting.

8. **Test error propagation**: Verify that errors are correctly propagated through domain clients.

9. **Test with location tracking**: Check that location information is preserved in errors.

10. **Create focused tests**: Each test should verify one specific piece of functionality.

11. **Keep error tests separate**: Create dedicated tests for error scenarios.

12. **Verify error details**: Check specific error fields like status, message, and location.

## Integration with Testing Frameworks

The MockApiClient is designed to work seamlessly with Rust's testing frameworks:

```rust
// Using with tokio test
#[tokio::test]
async fn my_test() {
    // Test code
}

// Using with standard test (when not using async)
#[test]
fn my_sync_test() {
    // Test code
}

// Using with feature-gated tests
#[cfg(feature = "reactive")]
#[tokio::test]
async fn my_reactive_test() {
    // Test code that requires the reactive feature
}
```

For more complex testing scenarios, you can also use test fixtures or setup/teardown patterns:

```rust
async fn setup() -> (Arc<Claude>, Arc<MockApiClient>) {
    // Setup code
    let mock_api = Arc::new(MockApiClient::new());
    let client = Arc::new(Claude::with_mock_api(
        "test-api-key",
        mock_api_to_handler(mock_api.clone()),
    ).with_model(ClaudeModel::Sonnet));
    
    (client, mock_api)
}

#[tokio::test]
async fn test_with_setup() {
    let (client, mock_api) = setup().await;
    // Test code
}
```

## Using with Domain-Specific Integration Tests

For testing complex workflows involving multiple domain clients:

```rust
#[tokio::test]
async fn test_multi_domain_workflow() {
    // Create mock API with multiple responses
    let mock_api = Arc::new(MockApiClient::new());
    
    // Configure different responses for different requests
    // Use a unique pattern in each request to disambiguate
    
    // Sentiment response
    let sentiment_request_marker = "sentiment_analysis_marker";
    let mut sentiment_response = create_sentiment_response("Positive", 0.95);
    mock_api.add_mock(ClaudeModel::Sonnet, sentiment_response);
    
    // Entity response
    let entity_request_marker = "entity_extraction_marker";
    let mut entity_response = create_entity_response(vec![
        ("Apple", "Organization"),
        ("Tim Cook", "Person")
    ]);
    mock_api.add_mock(ClaudeModel::Sonnet, entity_response);
    
    // Create client
    let client = Arc::new(Claude::with_mock_api(
        "test-api-key",
        mock_api_to_handler(mock_api.clone()),
    ).with_model(ClaudeModel::Sonnet));
    
    // Run multi-domain workflow
    let sentiment_client = client.sentiment();
    let entity_client = client.entity();
    
    // The actual tests would do something meaningful with these clients
    // This is just a simplified example
}
```

For more examples and details, see the test files in the repository, particularly:
- `tests/domain_mock_tests.rs`
- `tests/mock_api_client.rs`
- `tests/test_helpers.rs`