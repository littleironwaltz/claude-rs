use std::sync::Arc;
use claude_rs::{Claude, ClaudeModel, DomainClient, DomainOperations};
use claude_rs::types::{MessageRequest, ClaudeError};
use claude_rs::domains::{SentimentAnalysisClient, EntityExtractionClient};
// Not using async_trait in this file
use super::test_helpers::{
    setup_mock_with_json_response,
    // create_sentiment_response
};

// Initialize the test environment
use super::init;

/// Test integration between multiple domain clients in a single workflow
#[tokio::test]
async fn test_multi_domain_client_workflow() {
    // Initialize the test environment
    init();
    
    // Initialize the test environment
    
    // Create a client
    let client = Arc::new(Claude::new("test-api-key").with_model(ClaudeModel::Sonnet));
    
    // Get domain clients
    let sentiment_client = client.sentiment();
    let entity_client = client.entity();
    let code_client = client.code();
    let content_client = client.content();
    
    // Test that we can get all domain clients from the same Claude client
    // Since claude() returns a reference, we'll just compare the addresses directly
    let sentiment_claude = sentiment_client.claude() as *const Claude;
    let entity_claude = entity_client.claude() as *const Claude;
    let code_claude = code_client.claude() as *const Claude;
    let content_claude = content_client.claude() as *const Claude;
    
    assert_eq!(sentiment_claude, entity_claude);
    assert_eq!(sentiment_claude, code_claude);
    assert_eq!(sentiment_claude, content_claude);
    
    // Domain clients should have different domain names
    assert_ne!(sentiment_client.domain_name(), entity_client.domain_name());
    assert_ne!(sentiment_client.domain_name(), code_client.domain_name());
    assert_ne!(sentiment_client.domain_name(), content_client.domain_name());
}

/// Test integration with domain registry
#[tokio::test]
async fn test_domain_registry_integration() {
    // Initialize the test environment
    init();
    
    // Create a client
    let client = Arc::new(Claude::new("test-api-key").with_model(ClaudeModel::Sonnet));
    
    // Get domain clients
    let sentiment_client = client.sentiment();
    let entity_client = client.entity();
    
    // Since client.get_domain requires a domain name string, we'll use the domain names directly
    let sentiment_domain_name = sentiment_client.domain_name();
    let entity_domain_name = entity_client.domain_name();
    
    // The domain clients should already be in the registry from the domain() calls
    // Let's verify we can retrieve them
    let sentiment_via_domain = client.domains().sentiment();
    let entity_via_domain = client.domains().entity();
    
    // Add sentinel text to the domain names to ensure we're really testing the registry lookup
    // not just accidentally finding the instances we just created
    let sentinel_sentiment_name = format!("{}_test", sentiment_domain_name);
    let sentinel_entity_name = format!("{}_test", entity_domain_name);
    
    // Let's try a different approach to show how different clients can be registered
    struct CustomSentimentClient(Arc<SentimentAnalysisClient>);
    struct CustomEntityClient(Arc<EntityExtractionClient>);
    
    // Implement DomainClient for our custom wrappers
    impl DomainClient for CustomSentimentClient {
        fn domain_name(&self) -> &str {
            self.0.domain_name()
        }
    }
    
    impl DomainClient for CustomEntityClient {
        fn domain_name(&self) -> &str {
            self.0.domain_name()
        }
    }
    
    // Create custom wrappers
    let custom_sentiment = CustomSentimentClient(sentiment_via_domain);
    let custom_entity = CustomEntityClient(entity_via_domain);
    
    // Register using our custom wrappers
    client.domains().register(&sentinel_sentiment_name, custom_sentiment);
    client.domains().register(&sentinel_entity_name, custom_entity);
    
    // Use sentinel domain names to retrieve domain clients from registry
    if let Some(retrieved_sentiment) = client.get_domain(&sentinel_sentiment_name) {
        // The domain name should match the original, not our sentinel name
        assert_eq!(retrieved_sentiment.domain_name(), sentiment_client.domain_name());
    } else {
        panic!("Failed to retrieve sentiment client from registry");
    }
    
    if let Some(retrieved_entity) = client.get_domain(&sentinel_entity_name) {
        // The domain name should match the original, not our sentinel name
        assert_eq!(retrieved_entity.domain_name(), entity_client.domain_name());
    } else {
        panic!("Failed to retrieve entity client from registry");
    }
}

/// Test domain client with middleware
#[tokio::test]
async fn test_domain_client_with_middleware() {
    // Initialize the test environment
    init();
    
    // Create a sentiment response
    let sentiment_json = r#"{"sentiment": "Positive", "score": 0.95, "aspects": {}}"#;
    
    // Set up a client with a mock response
    let (client, _) = setup_mock_with_json_response(sentiment_json).await;
    
    // Since client is Arc<Claude>, we first need to clone and unwrap it
    let mut client_clone = (*client).clone();
    
    // Create a struct that implements RequestMiddleware
    use claude_rs::RequestMiddleware;
    
    struct SentimentAnalysisMiddleware;
    
    #[async_trait::async_trait]
    impl RequestMiddleware for SentimentAnalysisMiddleware {
        async fn process_request(&self, mut request: MessageRequest) -> Result<MessageRequest, ClaudeError> {
            request.system = Some("You are a helpful assistant specialized in sentiment analysis.".to_string());
            Ok(request)
        }
    }
    
    // Add the middleware to the client
    client_clone = client_clone.add_request_middleware(SentimentAnalysisMiddleware);
    let client = Arc::new(client_clone);
    
    // Get the domain client
    let sentiment_client = client.sentiment();
    
    // Test that the middleware is applied
    let result = sentiment_client.analyze_text("This is a great product!").await.unwrap();
    
    // Verify the result
    assert_eq!(result.sentiment, claude_rs::Sentiment::Positive);
}