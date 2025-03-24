// Test module organization
pub mod mock_api_client;
pub mod test_helpers;
// mock_client_adapter.rs is being deprecated and integrated into mock_api_client.rs
// Features will remain accessible until fully migrated
pub mod mock_client_adapter;
// Integration tests in separate module
pub mod integration;

// Make mock functionality available to other tests
pub use mock_api_client::{
    MockApiClient,
    create_sample_message_response,
    create_sample_delta_events,
    create_json_response,
    create_text_response,
    create_sentiment_response,
    create_entity_response,
    create_code_analysis_response,
    mock_api_to_handler
};

// Export all test helpers for better organization
pub use test_helpers::{
    DomainTester,
    test_sentiment, 
    test_entity, 
    test_content, 
    test_code,
    create_sentiment_response as create_test_sentiment_response,
    create_entity_response as create_test_entity_response,
    create_code_analysis_response as create_test_code_analysis_response,
    create_text_response as create_test_text_response,
    create_json_response as create_test_json_response,
    setup_mock_with_json_response,
    setup_mock_with_streaming_text,
    create_mock_stream_response
};

// Still maintain support for the MockClientAdapter trait for backward compatibility
pub use mock_client_adapter::MockClientAdapter;

/// Test macro for domain client functionality
/// 
/// # Arguments
/// * `$domain` - The domain client method on the Claude client (e.g., `sentiment`)
/// * `$method` - The method to test on the domain client
/// * `$input` - The input to the method
/// * `$mock_response` - The mock response to return
/// * `$expected` - The expected result
#[macro_export]
macro_rules! test_domain_client {
    ($domain:expr, $method:ident, $input:expr, $mock_response:expr, $expected:expr) => {
        #[tokio::test]
        async fn $method() {
            // Set up mock API client
            let mock_api = Arc::new(MockApiClient::new());
            mock_api.add_response(ClaudeModel::Sonnet, $mock_response);
            
            // Create client
            let client = Arc::new(Claude::with_mock_api(
                "test-api-key",
                mock_api_to_handler(mock_api.clone())
            ).with_model(ClaudeModel::Sonnet));
            
            // Get domain client
            let domain_client = client.$domain();
            
            // Call the method being tested
            let result = domain_client.$method($input).await.unwrap();
            
            // Verify the result
            assert_eq!(result, $expected);
            
            // Verify request history
            let requests = mock_api.get_request_history();
            assert_eq!(requests.len(), 1);
            assert!(requests[0].messages[0].content[0].to_string().contains(stringify!($input)));
        }
    };
}

/// Test macro for domain client error cases
/// 
/// # Arguments
/// * `$domain` - The domain client method on the Claude client (e.g., `sentiment`)
/// * `$method` - The method to test on the domain client
/// * `$input` - The input to the method
/// * `$error_type` - The expected error pattern to match against
#[macro_export]
macro_rules! test_domain_client_error {
    ($domain:expr, $method:ident, $input:expr, $error_type:pat) => {
        #[tokio::test]
        async fn $method() {
            // Set up mock API client
            let mock_api = Arc::new(MockApiClient::new());
            mock_api.add_error(ClaudeModel::Sonnet, ClaudeError::api_error(429, "Rate limit exceeded".to_string(), None));
            
            // Create client
            let client = Arc::new(Claude::with_mock_api(
                "test-api-key",
                mock_api_to_handler(mock_api.clone())
            ).with_model(ClaudeModel::Sonnet));
            
            // Get domain client
            let domain_client = client.$domain();
            
            // Call the method being tested
            let result = domain_client.$method($input).await;
            
            // Verify error
            assert!(matches!(result, Err($error_type)));
        }
    };
}