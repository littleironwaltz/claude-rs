
#[tokio::test]
async fn test_builder_basic() {
    let client = claude_rs::Claude::new("test_api_key");
    let _builder = client.message()
        .user_content("Hello, Claude")
        .system("You are a helpful assistant").unwrap()
        .max_tokens(100).unwrap()
        .temperature(0.7).unwrap();
    
    // We can't call send() because it would make a real API call,
    // but we can test the builder configuration
}

#[test]
fn test_message_building() {
    let client = claude_rs::Claude::new("test_api_key");
    let _builder = client.message()
        .user_content("First message")
        .assistant_content("Response")
        .user_content("Follow-up question");
    
    // In a real test, we would call prepare_request and check the request structure,
    // but it's private and would require mocking
}

#[test]
fn test_validation_failures() {
    let client = claude_rs::Claude::new("test_api_key");
    
    // Test temperature validation (outside 0.0-1.0 range)
    let result = client.message()
        .user_content("Hello")
        .temperature(1.5);
    assert!(result.is_err());
    
    // Test max_tokens validation (must be > 0)
    let result = client.message()
        .user_content("Hello")
        .max_tokens(0);
    assert!(result.is_err());
    
    // Test empty user message validation
    let result = client.message().user_message("");
    assert!(result.is_err());
}
