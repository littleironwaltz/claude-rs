use claude_rs::{Claude, ClaudeModel, MockApiHandler};
use claude_rs::types::*;
use std::sync::Arc;
use std::future::Future;
use std::pin::Pin;
// No need for this import

// Import test helpers
mod test_helpers;

use test_helpers::{
    setup_mock_with_json_response,
    // Only import what we use
};

// A simple mock API handler for testing
struct TestMockApiHandler {
    response: MessageResponse,
}

#[allow(dead_code)]
impl TestMockApiHandler {
    #[allow(dead_code)]
    fn new() -> Self {
        Self {
            response: MessageResponse {
                id: "msg_mock123".to_string(),
                model: "claude-3-sonnet-20240229".to_string(),
                r#type: "message".to_string(),
                role: Role::Assistant,
                content: vec![Content::Text { 
                    text: "This is a test mock response.".to_string() 
                }],
                usage: Usage {
                    input_tokens: 10,
                    output_tokens: 5,
                },
                stop_reason: Some("end_turn".to_string()),
                stop_sequence: None,
            }
        }
    }
    
    #[allow(dead_code)]
    fn with_text(mut self, text: &str) -> Self {
        self.response.content = vec![Content::Text { text: text.to_string() }];
        self
    }
}

impl MockApiHandler for TestMockApiHandler {
    fn process_request(&self, _request: MessageRequest) -> Pin<Box<dyn Future<Output = ClaudeResult<MessageResponse>> + Send>> {
        // Clone the response to avoid ownership issues
        let response = self.response.clone();
        Box::pin(async move {
            Ok(response)
        })
    }
    
    fn process_stream_request(&self, _request: MessageRequest) -> Pin<Box<dyn Future<Output = ClaudeResult<MessageStream>> + Send>> {
        // Create a simple stream with one event
        let event = DeltaEvent {
            event_type: "content_block_delta".to_string(),
            message: Some(DeltaMessage {
                id: "msg_mock123".to_string(),
                model: "claude-3-sonnet-20240229".to_string(),
                content: Some(vec![Content::Text {
                    text: "Streaming test response".to_string(),
                }]),
                stop_reason: None,
                stop_sequence: None,
                role: Some(Role::Assistant),
                type_field: Some("message".to_string()),
            }),
            usage: None,
            index: Some(0),
            delta: None,
        };
        
        // Create a stream that just returns this event
        let stream = futures::stream::once(async { Ok(event) });
        Box::pin(async move {
            Ok(Box::pin(stream) as MessageStream)
        })
    }
}

// Add From implementation for our test mock
impl From<TestMockApiHandler> for Arc<dyn MockApiHandler> {
    fn from(handler: TestMockApiHandler) -> Self {
        Arc::new(handler) as Arc<dyn MockApiHandler>
    }
}

#[tokio::test]
async fn test_with_mock_handler() {
    // Create a mock with custom response text
    let mock = TestMockApiHandler::new()
        .with_text(r#"```json
{"sentiment": "Positive", "score": 0.95, "aspects": {}}
```"#);
    
    // Create a Claude client with the mock
    let client = Claude::with_mock_api("test-api-key", mock);
    let client = client.with_model(ClaudeModel::Sonnet);
    
    // Use the client with the sentiment domain
    let sentiment_client = client.sentiment();
    
    // Test the analyze_text method
    let result = sentiment_client.analyze_text("This is a great product!").await.unwrap();
    
    // Verify the result
    assert_eq!(result.sentiment, claude_rs::Sentiment::Positive);
    assert!((result.score - 0.95).abs() < 0.001);
}

#[tokio::test]
async fn test_with_improved_helpers() {
    // Use the improved test helpers
    let (client, mock_api) = setup_mock_with_json_response(
        r#"{"sentiment": "Positive", "score": 0.95, "aspects": {}}"#
    ).await;
    
    // Use the sentiment domain client
    let sentiment_client = client.sentiment();
    
    // Test the analyze_text method
    let result = sentiment_client.analyze_text("This is a great product!").await.unwrap();
    
    // Verify the result
    assert_eq!(result.sentiment, claude_rs::Sentiment::Positive);
    assert!((result.score - 0.95).abs() < 0.001);
    
    // Verify that a request was made with the expected content
    let requests = mock_api.get_request_history();
    assert_eq!(requests.len(), 1);
    match &requests[0].messages[0].content[0] {
        Content::Text { text } => assert!(text.contains("This is a great product!")),
        _ => panic!("Expected Content::Text"),
    }
}

// Test CodeAssistanceClient with mock handler
#[tokio::test]
async fn test_code_assistance() {
    // Create a mock with JSON response for code analysis
    let mock = TestMockApiHandler::new()
        .with_text(r#"```json
{
    "issues": [
        {
            "line": 5,
            "severity": "Medium",
            "description": "Unused variable",
            "code": "let x = 5;"
        }
    ],
    "suggestions": [],
    "complexity_score": 3,
    "summary": "Code has minor issues"
}
```"#);
    
    // Create a Claude client with the mock
    let client = Claude::with_mock_api("test-api-key", mock);
    let client = client.with_model(ClaudeModel::Sonnet);
    
    // Use the client with the code domain
    let code_client = client.code();
    
    // Test the analyze_code method
    let result = code_client.analyze_code("let x = 5;", "javascript").await.unwrap();
    
    // Verify the result
    assert_eq!(result.issues.len(), 1);
    assert_eq!(result.issues[0].severity, claude_rs::IssueSeverity::Medium);
    assert_eq!(result.complexity_score, 3);
}