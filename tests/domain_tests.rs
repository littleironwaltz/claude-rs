use claude_rs::Claude;
use claude_rs::domains::*;
use claude_rs::domains::base::BaseDomainClient;
use std::sync::Arc;

#[tokio::test]
async fn test_domain_registry() {
    let client = Arc::new(Claude::new("test_api_key"));
    let registry = client.domains();
    
    // Test getting built-in domains
    let sentiment = registry.sentiment();
    assert_eq!(sentiment.domain_name(), "sentiment_analysis");
    
    let entity = registry.entity();
    assert_eq!(entity.domain_name(), "entity_extraction");
    
    let content = registry.content();
    assert_eq!(content.domain_name(), "content_generation");
    
    let code = registry.code();
    assert_eq!(code.domain_name(), "code_assistance");
    
    let translation = registry.translation();
    assert_eq!(translation.domain_name(), "translation");
    
    // Test non-existent domain with lock-free get
    let domain = registry.get("nonexistent");
    assert!(domain.is_none());
    
    // Test listing domains - should be empty since we haven't registered any custom ones
    let domains = registry.list_domains();
    assert!(domains.is_empty());
}

// Example of a custom domain client for testing
struct TestDomainClient {
    base: BaseDomainClient,
}

impl TestDomainClient {
    fn new(claude: Arc<Claude>) -> Self {
        Self { base: BaseDomainClient::new(claude, "test_domain") }
    }
}

impl DomainClient for TestDomainClient {
    fn domain_name(&self) -> &str {
        self.base.domain_name()
    }
}

impl ValidationOperations for TestDomainClient {}

impl DomainOperations for TestDomainClient {
    fn claude(&self) -> &Claude {
        self.base.claude()
    }
}

#[tokio::test]
async fn test_custom_domain_registration() {
    // Now with RwLock we can properly test domain registration
    
    let client = Arc::new(Claude::new("test_api_key"));
    let test_domain = TestDomainClient::new(client.clone());
    
    // Verify that the domain name works correctly
    assert_eq!(test_domain.domain_name(), "test_domain");
    
    // Register the custom domain (no longer async)
    let registry = client.domains();
    registry.register("test_domain", test_domain);
    
    // Now we can test retrieving the domain (no longer async)
    let retrieved_domain = registry.get("test_domain");
    assert!(retrieved_domain.is_some());
    assert_eq!(retrieved_domain.unwrap().domain_name(), "test_domain");
    
    // Test listing domains - should now include our custom domain
    let domains = registry.list_domains();
    assert_eq!(domains.len(), 1);
    assert!(domains.contains(&"test_domain".to_string()));
}
