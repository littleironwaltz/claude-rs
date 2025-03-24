use claude_rs::{Claude, ClaudeResult, ClaudeModel};
use claude_rs::domains::*;
use claude_rs::types::*;
use claude_rs::domains::base::BaseDomainClient;
use claude_rs::ResponseMiddleware;
use std::sync::Arc;
use tokio::test;
use async_trait::async_trait;

/// Simple test that verifies domain client creation
#[test]
async fn test_domain_client_creation() {
    // Create a client 
    let client = Claude::new("test-api-key")
        .with_model(ClaudeModel::Sonnet37);
    
    // Get domain clients
    let sentiment_client = client.sentiment();
    let entity_client = client.entity();
    let content_client = client.content();
    let code_client = client.code();
    
    // Verify domain client configurations
    assert_eq!(sentiment_client.domain_name(), "sentiment_analysis");
    assert_eq!(entity_client.domain_name(), "entity_extraction");
    assert_eq!(content_client.domain_name(), "content_generation");
    assert_eq!(code_client.domain_name(), "code_assistance");
}

/// Test that creates a custom domain client
#[test]
async fn test_custom_domain_client() {
    // Create a client
    let client = Arc::new(Claude::new("test-api-key")
        .with_model(ClaudeModel::Sonnet));
    
    // Create a custom domain client
    struct TranslationClient {
        base: BaseDomainClient,
    }
    
    impl TranslationClient {
        fn new(claude: Arc<Claude>) -> Self {
            Self {
                base: BaseDomainClient::new(claude, "translation"),
            }
        }
    }
    
    impl DomainClient for TranslationClient {
        fn domain_name(&self) -> &str {
            self.base.domain_name()
        }
    }
    
    impl ValidationOperations for TranslationClient {}
    
    impl DomainOperations for TranslationClient {
        fn claude(&self) -> &Claude {
            self.base.claude()
        }
    }
    
    // Create a translation client
    let translation_client = TranslationClient::new(client.clone());
    
    // Register in the domain registry
    client.domains().register("translation", translation_client);
}

/// Test middleware implementation
#[test]
async fn test_middleware_implementation() {
    // Create a client
    let client = Claude::new("test-api-key")
        .with_model(ClaudeModel::Sonnet);
    
    // Create a response middleware
    struct TestMiddleware;
    
    #[async_trait]
    impl ResponseMiddleware for TestMiddleware {
        async fn process_response(&self, response: MessageResponse) -> ClaudeResult<MessageResponse> {
            // Just return the response unmodified for this test
            Ok(response) 
        }
    }
    
    // Add the middleware
    client.add_response_middleware(TestMiddleware);
    
    // Test passes by reaching this point without errors
    // No explicit assertion needed
}