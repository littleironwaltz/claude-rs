use claude_rs::{RequestMiddleware, ResponseMiddleware};
use claude_rs::types::*;
use std::sync::Mutex;
use async_trait::async_trait;

// Example request middleware for testing
struct TestRequestMiddleware {
    headers: Mutex<Vec<String>>,
}

impl TestRequestMiddleware {
    fn new() -> Self {
        Self { headers: Mutex::new(Vec::new()) }
    }
    
    fn get_headers(&self) -> Vec<String> {
        self.headers.lock().unwrap().clone()
    }
}

#[async_trait]
impl RequestMiddleware for TestRequestMiddleware {
    async fn process_request(&self, mut request: MessageRequest) -> ClaudeResult<MessageRequest> {
        // Record that this middleware was called
        self.headers.lock().unwrap().push("X-Test-Header".to_string());
        
        // Modify the request in some way
        if let Some(ref mut sys) = request.system {
            *sys = format!("[Modified] {}", sys);
        } else {
            request.system = Some("[Added by middleware]".to_string());
        }
        
        Ok(request)
    }
}

// Example response middleware for testing
struct TestResponseMiddleware {
    processed: Mutex<bool>,
}

impl TestResponseMiddleware {
    fn new() -> Self {
        Self { processed: Mutex::new(false) }
    }
    
    fn was_processed(&self) -> bool {
        *self.processed.lock().unwrap()
    }
}

#[async_trait]
impl ResponseMiddleware for TestResponseMiddleware {
    async fn process_response(&self, mut response: MessageResponse) -> ClaudeResult<MessageResponse> {
        // Record that this middleware was called
        *self.processed.lock().unwrap() = true;
        
        // Modify the response content if it's text
        if let Some(Content::Text { text }) = response.content.first_mut() {
            *text = format!("[Modified] {}", text);
        }
        
        Ok(response)
    }
}

#[tokio::test]
async fn test_request_middleware() {
    // Create the middleware
    let middleware = TestRequestMiddleware::new();
    
    // Create a request
    let request = MessageRequest {
        model: "claude-3-sonnet-20240229".to_string(),
        messages: vec![],
        system: Some("Original system prompt".to_string()),
        temperature: None,
        max_tokens: None,
        tools: None,
        top_p: None,
        top_k: None,
        stop_sequences: vec![],
        stream: None,
    };
    
    // Process the request
    let processed = middleware.process_request(request).await.unwrap();
    
    // Verify modifications
    assert_eq!(processed.system, Some("[Modified] Original system prompt".to_string()));
    assert_eq!(middleware.get_headers(), vec!["X-Test-Header"]);
}

#[tokio::test]
async fn test_response_middleware() {
    // Create the middleware
    let middleware = TestResponseMiddleware::new();
    
    // Create a response
    let response = MessageResponse {
        id: "msg_123".to_string(),
        content: vec![Content::Text {
            text: "Original response".to_string(),
        }],
        model: "claude-3-sonnet-20240229".to_string(),
        role: Role::Assistant,
        stop_reason: None,
        stop_sequence: None,
        usage: Usage {
            input_tokens: 10,
            output_tokens: 5,
        },
        r#type: "message".to_string(),
    };
    
    // Process the response
    let processed = middleware.process_response(response).await.unwrap();
    
    // Verify modifications
    if let Content::Text { text } = &processed.content[0] {
        assert_eq!(text, "[Modified] Original response");
    } else {
        panic!("Expected Content::Text variant");
    }
    assert!(middleware.was_processed());
}
