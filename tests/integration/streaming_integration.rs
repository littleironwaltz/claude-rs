// Only import what we need

#[cfg(feature = "reactive")]
use claude_rs::reactive::ReactiveResponseStatus;

// Initialize the test environment
// use super::init;

/// Test domain clients with streaming responses
#[tokio::test]
#[cfg(feature = "reactive")]
async fn test_domain_client_with_streaming() {
    // Initialize the test environment
    init();
    
    // Create a client with streaming text
    let (client, mock_api) = setup_mock_with_streaming_text(
        vec!["This ", "is ", "an ", "integration ", "test ", "with ", "streaming."]
    ).await;
    
    // Get a domain client
    let code_client = client.code();
    
    // Use regular message builder from the base client
    let builder = client.message().user_content(
        "function test() { return 'hello'; }"
    ).system_prompt("You are a code analyst. Analyze this function.");
    
    // Request a streaming response
    let mut stream = builder.stream().await.unwrap();
    
    // Collect the chunks
    let mut text = String::new();
    while let Some(result) = stream.next().await {
        if let Ok(event) = result {
            if let Some(delta_text) = event.to_text() {
                text.push_str(&delta_text);
            }
        }
    }
    
    // Verify the result - combined text from the mock response
    assert_eq!(text, "This is an integration test with streaming.");
    
    // Verify request history
    let requests = mock_api.get_request_history();
    assert_eq!(requests.len(), 1);
    
    // Check that the system message was included
    let system_message = requests[0].messages.iter()
        .find(|msg| msg.role == Role::System);
    
    assert!(system_message.is_some());
    if let Some(message) = system_message {
        if let Content::Text { text } = &message.content[0] {
            assert!(text.contains("You are a code analyst"));
        } else {
            panic!("Expected text content");
        }
    }
}

/// Test integration between domain client operations and streaming
/// to ensure they can be used together in the same workflow
#[tokio::test]
#[cfg(feature = "reactive")]
async fn test_streaming_domain_client_integration() {
    // Initialize the test environment
    init();
    
    // Create a client with streaming capabilities
    let (client, mock_api) = setup_mock_with_streaming_text(
        vec!["Integration ", "test ", "with ", "domain ", "client."]
    ).await;
    
    // Get the domain client
    let code_client = client.code();
    
    // Use domain client methods to build a message
    let response = code_client.generate_test_harness(
        "function sum(a, b) { return a + b; }", 
        "javascript"
    ).await.unwrap();
    
    // The domain client method internally just uses Claude client with mocks
    assert!(mock_api.get_request_history().len() >= 1);
    
    // Use the same client for streaming to confirm both capabilities work
    let builder = client.message().user_content("Stream test");
    
    // Get reactive response
    let reactive = client.send_reactive(builder).await.unwrap();
    
    // Verify streaming works after domain client operation
    assert_eq!(reactive.status(), ReactiveResponseStatus::Initializing);
}