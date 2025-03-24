use claude_rs::{Claude, ClaudeModel};
use claude_rs::types::*;
// We use async_trait but import it directly where needed
use std::sync::Arc;

// Import test helpers
mod test_helpers;
mod mock_api_client;

// No test helpers are currently used directly in this file
use mock_api_client::{
    MockApiClient,
    mock_api_to_handler,
    // Only import what we use
    create_sentiment_response,
    create_entity_response,
    create_code_analysis_response
};

/// Creates a Claude client with the MockApiClient
/// 
/// This is a convenience function for tests that need a Claude client with a mock API.
pub fn create_mock_claude(
    api_key: &str, 
    model: ClaudeModel
) -> (Arc<Claude>, Arc<MockApiClient>) {
    let mock_api = Arc::new(MockApiClient::new().with_deterministic_timing());
    let client = Arc::new(Claude::with_mock_api(api_key, mock_api_to_handler(mock_api.clone()))
        .with_model(model));
    
    (client, mock_api)
}

/// Creates a Claude client with the MockApiClient and a pre-configured response
/// 
/// Uses the generic add_mock method to add a response to the mock API client
pub fn create_mock_claude_with_response(
    api_key: &str,
    model: ClaudeModel,
    response: MessageResponse
) -> (Arc<Claude>, Arc<MockApiClient>) {
    let (client, mock_api) = create_mock_claude(api_key, model.clone());
    mock_api.add_mock(model, response);
    
    (client, mock_api)
}

/// Creates a Claude client with the MockApiClient and a pre-configured streaming response
/// 
/// Uses the generic add_mock method with automatic conversion from Vec<DeltaEvent> to MockResponse
pub fn create_mock_claude_with_stream(
    api_key: &str,
    model: ClaudeModel,
    delta_events: Vec<DeltaEvent>
) -> (Arc<Claude>, Arc<MockApiClient>) {
    let (client, mock_api) = create_mock_claude(api_key, model.clone());
    mock_api.add_mock(model, delta_events);
    
    (client, mock_api)
}

/// Creates a Claude client with the MockApiClient and a pre-configured error
/// 
/// Uses the generic add_mock method with automatic conversion from (ClaudeError, bool) to MockResponse
pub fn create_mock_claude_with_error(
    api_key: &str,
    model: ClaudeModel,
    error: ClaudeError
) -> (Arc<Claude>, Arc<MockApiClient>) {
    let (client, mock_api) = create_mock_claude(api_key, model.clone());
    mock_api.add_mock(model, (error, false));
    
    (client, mock_api)
}

/// Creates a Claude client with the MockApiClient and a pre-configured streaming error
pub fn create_mock_claude_with_stream_error(
    api_key: &str,
    model: ClaudeModel,
    error: ClaudeError
) -> (Arc<Claude>, Arc<MockApiClient>) {
    let (client, mock_api) = create_mock_claude(api_key, model.clone());
    mock_api.add_mock(model, (error, true));
    
    (client, mock_api)
}

/// Setup a mock client for sentiment analysis testing
pub async fn setup_mock_for_sentiment(sentiment: &str, score: f32) -> (Arc<Claude>, Arc<MockApiClient>) {
    // Use create_mock_claude_with_response helper for better consistency
    create_mock_claude_with_response(
        "test-api-key",
        ClaudeModel::Sonnet,
        create_sentiment_response(sentiment, score)
    )
}

/// Setup a mock client for entity extraction testing
pub async fn setup_mock_for_entity_extraction(entities: Vec<(&str, &str)>) -> (Arc<Claude>, Arc<MockApiClient>) {
    // Use create_mock_claude_with_response helper for better consistency
    create_mock_claude_with_response(
        "test-api-key",
        ClaudeModel::Sonnet,
        create_entity_response(entities)
    )
}

/// Setup a mock client for code analysis testing
pub async fn setup_mock_for_code_analysis(issues: Vec<(&str, &str)>, score: u32) -> (Arc<Claude>, Arc<MockApiClient>) {
    // Use create_mock_claude_with_response helper for better consistency
    create_mock_claude_with_response(
        "test-api-key",
        ClaudeModel::Sonnet,
        create_code_analysis_response(issues, score)
    )
}

/// Test client integration with domain client
#[tokio::test]
async fn test_client_domain_integration() {
    // Setup client with JSON response
    let (client, _) = setup_mock_for_sentiment("Positive", 0.95).await;
    
    // Get domain client
    let sentiment_client = client.sentiment();
    
    // Use domain client
    let result = sentiment_client.analyze_text("This is great!").await.unwrap();
    
    // Verify result
    assert_eq!(result.sentiment, claude_rs::Sentiment::Positive);
}

/// Test client integration with middleware and domain client
#[tokio::test]
async fn test_client_middleware_integration() {
    // Setup client with JSON response
    let (client, mock_api) = setup_mock_for_entity_extraction(vec![("John", "Person")]).await;
    
    // Since client is Arc<Claude>, we first need to clone and unwrap it
    let mut client_clone = (*client).clone();
    
    // Create a struct that implements RequestMiddleware
    use claude_rs::RequestMiddleware;
    
    struct EntityExtractionMiddleware;
    
    #[async_trait::async_trait]
    impl RequestMiddleware for EntityExtractionMiddleware {
        async fn process_request(&self, mut request: MessageRequest) -> Result<MessageRequest, ClaudeError> {
            // Add test metadata to the request
            request.system = Some("You are an entity extraction specialist.".to_string());
            Ok(request)
        }
    }
    
    // Add middleware
    client_clone = client_clone.add_request_middleware(EntityExtractionMiddleware);
    let client = Arc::new(client_clone);
    
    // Get domain client
    let entity_client = client.entity();
    
    // Use domain client
    // Extract entities from text through a prompt
    let _message = client.message()
        .user_content("John went to the store.")
        .send()
        .await
        .unwrap();
        
    // Get the entities from the response
    let entities = entity_client.extract_from_text("John went to the store.").await.unwrap();
    
    // Verify result - just check that we got a response
    assert!(!entities.is_empty());
    
    // Verify middleware was applied
    let requests = mock_api.get_request_history();
    assert!(requests[0].system.as_ref().unwrap().contains("entity extraction specialist"));
}

/// Test feature-gated functionality
#[tokio::test]
#[cfg(feature = "reactive")]
async fn test_reactive_feature_is_available() {
    // Setup client with streaming response
    let (client, _) = create_mock_claude_with_stream(
        "test-api-key",
        ClaudeModel::Sonnet,
        create_mock_stream_response(vec!["Test"], true)
    );
    
    // Create a message builder
    let builder = client.message().user_content("Test reactive");
    
    // This should compile only if the reactive feature is enabled
    let _reactive = client.send_reactive(builder).await.unwrap();
    assert!(true, "Reactive feature is available");
}

/// Test client configuration
#[tokio::test]
async fn test_client_configuration() {
    let client = Claude::new("test-api-key")
        .with_model(ClaudeModel::Sonnet)
        .with_base_url("https://custom-api.example.com");
    
    // Verify configuration
    // Note: model() method doesn't exist, so we can only check the base_url
    assert_eq!(client.base_url, "https://custom-api.example.com");
}