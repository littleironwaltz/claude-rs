use claude_rs::Claude;
use std::sync::Arc;
use claude_rs::ClaudeModel;
use claude_rs::types::ClaudeError;

#[tokio::test]
async fn test_client_initialization() {
    let client = Claude::new("test_api_key");
    assert_eq!(client.base_url, "https://api.anthropic.com/v1");
}

#[tokio::test]
async fn test_client_domain_methods() {
    let client = Claude::new("test_api_key");
    
    // Test domain client retrieval methods
    let _sentiment_client = client.sentiment();
    let _entity_client = client.entity();
    let _content_client = client.content();
    let _code_client = client.code();
    
    // Just test that we can create the clients without errors
}

// Import necessary modules from parent
mod mock_api_client;
mod test_helpers;

use mock_api_client::{
    MockApiClient, 
    mock_api_to_handler, 
    create_sample_message_response
};
// No need for this import

#[tokio::test]
#[cfg(feature = "reactive")]
async fn test_streaming_response() {
    // Use the improved helper function
    let (client, _) = setup_mock_with_streaming_text(
        vec!["This ", "is ", "a ", "test", " response"]
    ).await;
    
    // Create a message builder
    let builder = client.message()
        .user_content("Test streaming message");
    
    // Request a streaming response
    let mut stream = builder.stream().await.unwrap();
    
    // Collect text from the stream
    use futures::StreamExt;
    let mut text = String::new();
    while let Some(result) = stream.next().await {
        if let Ok(event) = result {
            if let Some(delta_text) = event.to_text() {
                text.push_str(&delta_text);
            }
        }
    }
    
    // Verify the result
    assert_eq!(text, "This is a test response");
}

#[tokio::test]
async fn test_client_with_different_models() {
    let mock_api = Arc::new(MockApiClient::new());
    
    // Configure mock with different responses for different models
    let mut haiku_response = create_sample_message_response();
    haiku_response.model = "claude-3-haiku-20240307".to_string();
    
    let mut opus_response = create_sample_message_response();
    opus_response.model = "claude-3-opus-20240229".to_string();
    
    mock_api.add_response(ClaudeModel::Haiku, haiku_response);
    mock_api.add_response(ClaudeModel::Opus, opus_response);
    
    // Test with Haiku model
    let haiku_client = Claude::with_mock_api(
        "test_api_key",
        mock_api_to_handler(mock_api.clone()),
    ).with_model(ClaudeModel::Haiku);
    
    let haiku_builder = haiku_client.message().user_content("Test message");
    let haiku_response = haiku_builder.send().await.unwrap();
    assert_eq!(haiku_response.model, "claude-3-haiku-20240307");
    
    // Test with Opus model
    let opus_client = Claude::with_mock_api(
        "test_api_key",
        mock_api_to_handler(mock_api.clone()),
    ).with_model(ClaudeModel::Opus);
    
    let opus_builder = opus_client.message().user_content("Test message");
    let opus_response = opus_builder.send().await.unwrap();
    assert_eq!(opus_response.model, "claude-3-opus-20240229");
    
    // Verify request history
    let requests = mock_api.get_request_history();
    assert_eq!(requests.len(), 2);
}

#[tokio::test]
async fn test_client_with_error_handling() {
    let mock_api = Arc::new(MockApiClient::new());
    
    // Configure mock to return an error for a specific model
    mock_api.add_mock(
        ClaudeModel::Haiku, 
        (ClaudeError::api_error("Rate limit exceeded", Some(429), None, None), false)
    );
    
    // Create a client with the Haiku model and the mock API
    let client = Claude::with_mock_api(
        "test_api_key",
        mock_api_to_handler(mock_api.clone())
    ).with_model(ClaudeModel::Haiku);
    
    let builder = client.message().user_content("Test message");
    
    // Attempt to send the message, expect an error
    let result = builder.send().await;
    assert!(result.is_err());
    
    // Verify the error type
    if let Err(ClaudeError::ApiError { status, .. }) = result {
        assert_eq!(status, 429);
    } else {
        panic!("Expected ApiError with status 429");
    }
}