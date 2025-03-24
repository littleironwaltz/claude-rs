use claude_rs::{Claude, DomainClient, DomainOperations};

#[tokio::test]
async fn test_sentiment_client() {
    let client = Claude::new("test-api-key");
    let sentiment_client = client.sentiment();
    
    // Verify domain name
    assert_eq!(sentiment_client.domain_name(), "sentiment_analysis");
    
    // Verify we can get a reference to the Claude client
    let _ = sentiment_client.claude();
}

#[tokio::test]
async fn test_entity_client() {
    let client = Claude::new("test-api-key");
    let entity_client = client.entity();
    
    // Verify domain name
    assert_eq!(entity_client.domain_name(), "entity_extraction");
    
    // Verify we can get a reference to the Claude client
    let _ = entity_client.claude();
}

#[tokio::test]
async fn test_content_client() {
    let client = Claude::new("test-api-key");
    let content_client = client.content();
    
    // Verify domain name
    assert_eq!(content_client.domain_name(), "content_generation");
    
    // Verify we can get a reference to the Claude client
    let _ = content_client.claude();
}

#[tokio::test]
async fn test_code_client() {
    let client = Claude::new("test-api-key");
    let code_client = client.code();
    
    // Verify domain name
    assert_eq!(code_client.domain_name(), "code_assistance");
    
    // Verify we can get a reference to the Claude client
    let _ = code_client.claude();
}