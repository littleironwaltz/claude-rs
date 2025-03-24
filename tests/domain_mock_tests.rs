use claude_rs::{Claude, ClaudeError, ClaudeModel, request_error, domain_error};
use claude_rs::domains::*;
use claude_rs::types::*;
use std::sync::Arc;
use tokio::test;

// Import modules with full paths
mod mock_api_client;
mod test_helpers;
use mock_api_client::{
    MockApiClient, 
    create_sample_message_response,
    create_code_analysis_response,
    mock_api_to_handler
};

// Define helper functions locally in this file
fn create_mock_claude_with_response(
    api_key: &str,
    model: ClaudeModel,
    response: MessageResponse
) -> (Arc<Claude>, Arc<MockApiClient>) {
    let mock_api = Arc::new(MockApiClient::new());
    mock_api.add_response(model.clone(), response);
    
    let client = Arc::new(Claude::with_mock_api(
        api_key,
        mock_api_to_handler(mock_api.clone())
    ).with_model(model));
    
    (client, mock_api)
}

fn create_mock_claude_with_error(
    api_key: &str,
    model: ClaudeModel,
    error: ClaudeError
) -> (Arc<Claude>, Arc<MockApiClient>) {
    let mock_api = Arc::new(MockApiClient::new());
    mock_api.add_error(model.clone(), error);
    
    let client = Arc::new(Claude::with_mock_api(
        api_key,
        mock_api_to_handler(mock_api.clone())
    ).with_model(model));
    
    (client, mock_api)
}

// Removed unused function that was causing warning

// Helper function to verify that a request contains expected content in the user message
#[allow(dead_code)]
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

// Test SentimentAnalysisClient with our new test helpers
#[test]
async fn test_sentiment_analysis_client() {
    // Use test_helpers instead of manual mock setup
    use crate::test_helpers::{test_sentiment, create_sentiment_response};
    
    // Get a test helper with pre-configured mock
    let tester = test_sentiment();
    
    // Mock a sentiment response using the helper
    tester.mock_response(
        ClaudeModel::Sonnet,
        create_sentiment_response("Positive", 0.92)
    );
    
    // Test analyze_text method
    let result = tester.domain_client
        .analyze_text("I love this product! It works incredibly well.")
        .await
        .unwrap();
    
    // Verify the result
    assert_eq!(result.sentiment, Sentiment::Positive);
    assert!(result.score > 0.9);
    assert!(result.aspects.is_empty()); // Our mock helper doesn't add aspects by default
    
    // Verify the request contains correct sentiment analysis text using simplified helpers
    tester.assert_request_contains("sentiment");
    tester.assert_request_contains("I love this product");
}

// Test EntityExtractionClient with our new test helpers
#[test]
async fn test_entity_extraction_client() {
    // Use test_helpers instead of manual mock setup
    use crate::test_helpers::{test_entity, create_entity_response};
    
    // Get a test helper with pre-configured mock
    let tester = test_entity();
    
    // Define entities to extract
    let entities = vec![
        ("Apple", "Organization"),
        ("Steve Jobs", "Person"),
        ("California", "Location")
    ];
    
    // Mock an entity response
    tester.mock_response(
        ClaudeModel::Sonnet,
        create_entity_response(entities)
    );
    
    // Test input text
    let input_text = "Apple was founded by Steve Jobs and is based in California.";
    
    // Test extract_from_text method
    let result = tester.domain_client
        .extract_from_text(input_text)
        .await
        .unwrap();
    
    // Verify the result
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].text, "Apple");
    assert_eq!(result[0].entity_type, EntityType::Organization);
    assert_eq!(result[1].text, "Steve Jobs");
    assert_eq!(result[1].entity_type, EntityType::Person);
    assert_eq!(result[2].text, "California");
    assert_eq!(result[2].entity_type, EntityType::Location);
    
    // Verify the request contains the input text using our simplified helper
    tester.assert_request_contains(input_text);
    
    // Test filtering by entity type
    let persons = tester.domain_client.of_type(&result, &EntityType::Person);
    assert_eq!(persons.len(), 1);
    assert_eq!(persons[0].text, "Steve Jobs");
}

// Test ContentGenerationClient with MockApiClient
#[test]
async fn test_content_generation_client() {
    // Create and configure mock API client
    let mock_api = Arc::new(MockApiClient::new());
    
    // Configure a sample response for blog post generation
    let mut response = create_sample_message_response();
    response.content = vec![Content::Text { 
        text: "# The Future of AI\n\nArtificial intelligence has come a long way in recent years...".to_string()
    }];
    mock_api.add_mock(ClaudeModel::Sonnet, response.clone());
    
    // Create a Claude client with the mock API
    let client = Claude::with_mock_api("test-api-key", mock_api_to_handler(mock_api.clone()))
        .with_model(ClaudeModel::Sonnet);
        
    // Get the content client
    let content_client = client.content();
    
    // Test blog_post method
    let result = content_client.blog_post("The Future of AI", Some("informative".to_string()), Some(500)).await.unwrap();
    
    // Verify the result - string result should contain these substrings
    assert!(result.contains("The Future of AI"));
    assert!(result.contains("Artificial intelligence"));
    
    // Verify the request history
    let requests = mock_api.get_request_history();
    assert_eq!(requests.len(), 1);
    
    // Verify that the user message contains blog post related text
    let user_message = &requests[0].messages[0];
    assert_eq!(user_message.role, Role::User);
    if let Some(Content::Text { text }) = user_message.content.first() {
        assert!(text.contains("The Future of AI"));
    } else {
        panic!("User message doesn't contain text content");
    }
    
    // Reset the request history
    mock_api.clear_request_history();
    
    // Configure a sample response for product description
    let mut product_response = create_sample_message_response();
    product_response.content = vec![Content::Text { 
        text: "Introducing the AirFlow Pro, the revolutionary air purifier that transforms your living space...".to_string()
    }];
    mock_api.add_mock(ClaudeModel::Sonnet, product_response);
    
    // Test product_description method
    let features = vec![
        "HEPA filtration".to_string(), 
        "Silent operation".to_string(),
        "App control".to_string()
    ];
    
    let result = content_client.product_description(
        "AirFlow Pro Air Purifier", 
        features, 
        Some("health-conscious homeowners".to_string()), 
        Some(200)
    ).await.unwrap();
    
    // Verify the result
    assert!(result.contains("AirFlow Pro"));
    assert!(result.contains("revolutionary air purifier"));
    
    // Verify the request history
    let requests = mock_api.get_request_history();
    assert_eq!(requests.len(), 1);
    
    // Verify that the user message contains product description related text
    let user_message = &requests[0].messages[0];
    assert_eq!(user_message.role, Role::User);
    if let Some(Content::Text { text }) = user_message.content.first() {
        assert!(text.contains("AirFlow Pro Air Purifier"));
        assert!(text.to_lowercase().contains("hepa filtration"));
    } else {
        panic!("User message doesn't contain text content");
    }
}

// Test CodeAssistanceClient with MockApiClient
#[test]
async fn test_code_assistance_client() {
    // Use helper function to create code analysis response
    let issues = vec![
        ("Unused variable 'result'", "Medium"),
        ("Potential null reference exception", "High")
    ];
    
    let (client, mock_api) = create_mock_claude_with_response(
        "test-api-key",
        ClaudeModel::Sonnet,
        create_code_analysis_response(issues, 3)
    );
    
    // Get the code client
    let code_client = client.code();
    
    // Sample code to analyze
    let code_sample = r#"
        function processUser(user) {
            console.log("Processing user");
            
            let result = calculate();
            
            // This could cause an error if user is null
            console.log(user.getName().toLowerCase());
            
            return true;
        }
    "#;
    
    // Test analyze_code method
    let result = code_client.analyze_code(code_sample, "javascript").await.unwrap();
    
    // Verify the result
    assert_eq!(result.issues.len(), 2);
    assert_eq!(result.issues[0].description, "Unused variable 'result'");
    assert_eq!(result.issues[0].severity, IssueSeverity::Medium);
    assert_eq!(result.issues[1].description, "Potential null reference exception");
    assert_eq!(result.issues[1].severity, IssueSeverity::High);
    assert_eq!(result.complexity_score, 3);
    
    // Verify the request history
    let requests = mock_api.get_request_history();
    assert_eq!(requests.len(), 1);
    
    // Verify that the user message contains code analysis related text
    let user_message = &requests[0].messages[0];
    assert_eq!(user_message.role, Role::User);
    if let Some(Content::Text { text }) = user_message.content.first() {
        assert!(text.contains("javascript"));
        assert!(text.contains("function processUser"));
    } else {
        panic!("User message doesn't contain text content");
    }
    
    // Clear request history for next test
    mock_api.clear_request_history();
    
    // Configure a sample response for code documentation
    let mut doc_response = create_sample_message_response();
    doc_response.content = vec![Content::Text { 
        text: "/**\n * Processes a user object\n * @param {User} user - The user to process\n * @returns {boolean} - Processing success status\n */".to_string()
    }];
    mock_api.add_mock(ClaudeModel::Sonnet, doc_response);
    
    // Test generate_docs method
    let docs = code_client.generate_docs(code_sample, "javascript", Some("JSDoc".to_string())).await.unwrap();
    
    // Verify the result
    assert!(docs.contains("@param"));
    assert!(docs.contains("@returns"));
    
    // Verify the request history
    let requests = mock_api.get_request_history();
    assert_eq!(requests.len(), 1);
    
    // Verify that the user message contains documentation generation related text
    let user_message = &requests[0].messages[0];
    assert_eq!(user_message.role, Role::User);
    if let Some(Content::Text { text }) = user_message.content.first() {
        assert!(text.contains("JSDoc"));
        assert!(text.contains("function processUser"));
    } else {
        panic!("User message doesn't contain text content");
    }
}

// Test error handling in domain clients with enhanced error structure
#[test]
async fn test_domain_client_error_handling() {
    // Create a mock client with error response using the improved error helpers with location
    let loc = concat!(file!(), ":", line!());
    let error = ClaudeError::api_error(
        "Rate limit exceeded", 
        Some(429), 
        None,
        Some(loc)
    );
    
    let (client, _) = create_mock_claude_with_error(
        "test-api-key",
        ClaudeModel::Sonnet,
        error
    );
        
    // Get all domain clients
    let sentiment_client = client.sentiment();
    let entity_client = client.entity();
    let content_client = client.content();
    let code_client = client.code();
    
    // Test that errors are properly propagated through domain clients
    let sentiment_result = sentiment_client.analyze_text("Test").await;
    assert!(sentiment_result.is_err());
    match sentiment_result {
        Err(ClaudeError::ApiError { status, location, .. }) => {
            assert_eq!(status, 429, "Expected status code 429 (rate limit) but got {}", status);
            // Verify that location was preserved in the error
            assert!(location.is_some(), "Location information missing in error");
            assert_eq!(location.unwrap(), loc);
        },
        Err(other_error) => {
            panic!("Expected ApiError with status 429, but got different error: {:?}", other_error);
        },
        _ => panic!("Expected error but got success"),
    }
    
    // Test that our new macros work for error creation
    let macro_error = request_error!("This is a test error");
    assert!(macro_error.location().is_some());
    
    let domain_macro_error = domain_error!("test_domain", "This is a domain test error");
    assert!(domain_macro_error.location().is_some());
    
    // Test errors propagate through all domain clients
    let entity_result = entity_client.extract_from_text("Test").await;
    assert!(entity_result.is_err());
    
    let content_result = content_client.blog_post("Test", None, None).await;
    assert!(content_result.is_err());
    
    let code_result = code_client.analyze_code("function test() {}", "javascript").await;
    assert!(code_result.is_err());
}

// Test streaming with domain clients (if applicable)
#[cfg(feature = "reactive")]
#[tokio::test]
async fn test_domain_client_streaming() {
    use futures::StreamExt;
    use crate::mock_api_client::create_sample_delta_events;
    
    // Create and configure mock API client
    let mock_api = Arc::new(MockApiClient::new());
    
    // Configure a streaming response using the improved add_mock method
    mock_api.add_mock(ClaudeModel::Sonnet, create_sample_delta_events());
    
    // Create a Claude client with the mock API
    let client = Claude::with_mock_api("test-api-key", mock_api_to_handler(mock_api.clone()))
        .with_model(ClaudeModel::Sonnet);
        
    // We don't need the domain client in this approach,
    // since we're using the main client directly
    let _code_client = client.code();
    
    // Example code for the streaming operation
    let code_sample = "function test() { return 'hello'; }";
    
    // Instead of using dynamic dispatch, directly use the client
    // The original code was trying to use dynamic dispatch with a trait that has generic methods,
    // which is not object-safe
    
    // Start a streaming operation directly with the client
    let builder = client.message().user_content(
        &format!("Analyze this code: {}", code_sample)
    );
    
    let mut stream = builder.stream().await.unwrap();
    
    // Collect all chunks
    let mut text_chunks = Vec::new();
    let mut final_event_detected = false;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.unwrap();
        if let Some(text) = chunk.to_text() {
            text_chunks.push(text);
        }
        
        // Also test the is_final helper
        if chunk.is_final() {
            final_event_detected = true;
        }
    }
    
    // Verify we received content
    assert!(!text_chunks.is_empty());
    let full_text = text_chunks.join("");
    assert!(full_text.contains("sample streaming response"));
    
    // Verify we detected the final event using our helper method
    assert!(final_event_detected, "Expected to detect a final event with is_final()");
    
    // Verify the request history
    let requests = mock_api.get_request_history();
    assert_eq!(requests.len(), 1);
}