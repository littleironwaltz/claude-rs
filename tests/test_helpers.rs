use claude_rs::{Claude, ClaudeModel};
use claude_rs::types::{MessageResponse, Role, Content, Usage, DeltaEvent, DeltaMessage, MessageRequest, ClaudeResult, ClaudeError, Delta};
use claude_rs::domains::{
    SentimentAnalysisClient, EntityExtractionClient, 
    ContentGenerationClient, CodeAssistanceClient
};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::pin::Pin;
use std::future::Future;
use std::time::Duration;
use futures::Stream;

// Type alias for message stream result to reduce complexity
type MessageStreamResult = ClaudeResult<Pin<Box<dyn Stream<Item = Result<DeltaEvent, ClaudeError>> + Send>>>;

/// A domain-specific tester for more consistent testing
#[allow(dead_code)]
pub struct DomainTester<T> {
    #[allow(dead_code)]
    pub client: Arc<Claude>,
    pub mock_api: Arc<MockApiClient>,
    pub domain_client: Arc<T>,
}

#[allow(dead_code)]
impl<T> DomainTester<T> {
    pub fn new(domain_client: Arc<T>, client: Arc<Claude>, mock_api: Arc<MockApiClient>) -> Self {
        Self {
            client,
            mock_api,
            domain_client,
        }
    }
    
    /// Mock response with simple setup
    pub fn mock_response(&self, model: ClaudeModel, response: MessageResponse) {
        self.mock_api.add_mock(model, response);
    }
    
    /// Request verification helper
    pub fn assert_request_contains(&self, expected_content: &str) -> bool {
        let requests = self.mock_api.get_request_history();
        if requests.is_empty() {
            return false;
        }
        
        let user_messages = requests[0].messages.iter()
            .filter(|msg| msg.role == Role::User)
            .collect::<Vec<_>>();
        
        if user_messages.is_empty() {
            return false;
        }
        
        let content_text = match &user_messages[0].content[0] {
            Content::Text { text } => text,
            _ => return false,
        };
        
        content_text.contains(expected_content)
    }
    
    /// Mock streaming responses
    pub fn mock_stream(&self, model: ClaudeModel, text_chunks: Vec<&str>) {
        let stream_events = create_mock_stream_response(text_chunks, true);
        self.mock_api.add_stream_response(model, stream_events);
    }
}

/// A simple mock API client for testing - should be integrated with mock_api_client.rs
pub struct MockApiClient {
    responses: Mutex<HashMap<String, MessageResponse>>,
    stream_responses: Mutex<HashMap<String, Vec<DeltaEvent>>>,
    requests: Mutex<Vec<MessageRequest>>,
    response_delay: Mutex<Option<Duration>>,
}

#[allow(dead_code)]
impl Default for MockApiClient {
    fn default() -> Self {
        Self {
            responses: Mutex::new(HashMap::new()),
            stream_responses: Mutex::new(HashMap::new()),
            requests: Mutex::new(Vec::new()),
            response_delay: Mutex::new(None),
        }
    }
}

#[allow(dead_code)]
impl MockApiClient {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_deterministic_timing(self) -> Self {
        // Set a predictable timing for tests
        *self.response_delay.lock().unwrap() = Some(Duration::from_millis(10));
        self
    }
    
    pub fn add_mock(&self, model: ClaudeModel, response: MessageResponse) {
        let mut responses = self.responses.lock().unwrap();
        responses.insert(model.as_str().to_string(), response);
    }
    
    pub fn add_stream_response(&self, model: ClaudeModel, events: Vec<DeltaEvent>) {
        let mut stream_responses = self.stream_responses.lock().unwrap();
        stream_responses.insert(model.as_str().to_string(), events);
    }
    
    pub fn get_request_history(&self) -> Vec<MessageRequest> {
        self.requests.lock().unwrap().clone()
    }
    
    pub fn with_delay(&self, delay: Duration) -> &Self {
        *self.response_delay.lock().unwrap() = Some(delay);
        self
    }
    
    fn process_request_internal(&self, request: MessageRequest) -> ClaudeResult<MessageResponse> {
        // Add request to history
        self.requests.lock().unwrap().push(request.clone());
        
        // Get response for model
        let responses = self.responses.lock().unwrap();
        if let Some(response) = responses.get(&request.model) {
            Ok(response.clone())
        } else {
            Err(ClaudeError::api_error(
                format!("No mock response found for model: {}", request.model),
                Some(404),
                None, 
                Some("test_helper.rs:mock_api")
            ))
        }
    }
    
    // Update the existing method to be async
    pub async fn process_stream_request(&self, request: MessageRequest) -> MessageStreamResult {
        // Process the stream request without holding mutexes across awaits
        let stream_result = self.process_stream_request_internal(request.clone());
        
        // Get delay value before awaiting
        let delay_to_apply = self.response_delay.lock()
            .ok()
            .and_then(|guard| *guard);
        
        // Apply delay if configured
        if let Some(delay) = delay_to_apply {
            tokio::time::sleep(delay).await;
        }
        
        stream_result
    }
    
    // Create a new internal method that doesn't use await
    pub fn process_stream_request_internal(&self, request: MessageRequest) -> MessageStreamResult {
        // Add request to history
        if let Ok(mut requests) = self.requests.lock() {
            requests.push(request.clone());
        }
        
        // Get stream response for model
        if let Ok(stream_responses) = self.stream_responses.lock() {
            if let Some(events) = stream_responses.get(&request.model) {
                let events = events.clone();
                let (tx, rx) = tokio::sync::mpsc::channel(10);
                
                tokio::spawn(async move {
                    for event in events {
                        // Add a small delay between events for realism
                        tokio::time::sleep(Duration::from_millis(20)).await;
                        
                        if tx.send(Ok(event)).await.is_err() {
                            break;
                        }
                    }
                });
                
                // Create a stream from the receiver
                let stream = futures::stream::unfold(rx, |mut rx| async move {
                    rx.recv().await.map(|item| (item, rx))
                });
                
                return Ok(Box::pin(stream));
            }
        }
        
        // If we reach here, either we couldn't get the lock or no response was found
        Err(ClaudeError::api_error(
            format!("No mock stream response found for model: {}", request.model),
            Some(404),
            None, 
            Some("test_helper.rs:mock_api")
        ))
    }
}

/// Helper function to convert MockApiClient to dyn MockApiHandler
#[allow(dead_code)]
pub fn mock_api_to_handler(mock: Arc<MockApiClient>) -> Arc<dyn claude_rs::client::MockApiHandler + 'static> {
    mock as Arc<dyn claude_rs::client::MockApiHandler>
}

/// Implement MockApiHandler from client.rs for our test client
impl claude_rs::client::MockApiHandler for MockApiClient {
    fn process_request(&self, request: MessageRequest) -> Pin<Box<dyn Future<Output = ClaudeResult<MessageResponse>> + Send>> {
        // Clone necessary data for the async block
        let result = self.process_request_internal(request);
        
        // Get the delay before entering the async block
        let delay_option = if let Ok(guard) = self.response_delay.lock() {
            *guard
        } else {
            None
        };
        
        // Return a future that resolves to the response
        Box::pin(async move {
            // Apply delay if configured
            if let Some(delay) = delay_option {
                tokio::time::sleep(delay).await;
            }
            result
        })
    }
    
    fn process_stream_request(&self, request: MessageRequest) -> Pin<Box<dyn Future<Output = ClaudeResult<Pin<Box<dyn Stream<Item = Result<DeltaEvent, ClaudeError>> + Send>>>> + Send>> {
        // First clone the request history structures outside the async block
        let _request_clone = request.clone(); // Prefix with underscore to indicate it's intentionally unused
        
        // Create a stream result that doesn't hold the mutex across awaits
        let stream_result = self.process_stream_request_internal(request);
        
        Box::pin(async move {
            // Process the request and return the stream
            stream_result
        })
    }
}

impl Clone for MockApiClient {
    fn clone(&self) -> Self {
        Self {
            responses: Mutex::new(self.responses.lock().unwrap().clone()),
            stream_responses: Mutex::new(self.stream_responses.lock().unwrap().clone()),
            requests: Mutex::new(self.requests.lock().unwrap().clone()),
            response_delay: Mutex::new(*self.response_delay.lock().unwrap()),
        }
    }
}

/// Helper to create a mock stream response with text chunks
#[allow(dead_code)]
pub fn create_mock_stream_response(
    text_chunks: Vec<&str>, 
    include_final_event: bool
) -> Vec<DeltaEvent> {
    let mut events = vec![];
    
    // Add start event
    events.push(DeltaEvent {
        event_type: "message_start".to_string(),
        message: Some(DeltaMessage {
            id: "msg_mock123".to_string(),
            model: "claude-3-sonnet-20240229".to_string(),
            content: None,
            stop_reason: None,
            stop_sequence: None,
            role: Some(Role::Assistant),
            type_field: Some("message".to_string()),
        }),
        delta: None,
        usage: None,
        index: Some(0),
    });
    
    // Add text delta events
    for (i, chunk) in text_chunks.iter().enumerate() {
        events.push(DeltaEvent {
            event_type: "content_block_delta".to_string(),
            message: Some(DeltaMessage {
                id: "msg_mock123".to_string(),
                model: "claude-3-sonnet-20240229".to_string(),
                content: Some(vec![Content::Text {
                    text: chunk.to_string(),
                }]),
                stop_reason: None,
                stop_sequence: None,
                role: Some(Role::Assistant),
                type_field: Some("message".to_string()),
            }),
            delta: Some(Delta {
                text: Some(chunk.to_string()),
                stop_reason: None,
                stop_sequence: None,
            }),
            usage: None,
            index: Some(i as u32 + 1),
        });
    }
    
    // Add final event if requested
    if include_final_event {
        events.push(DeltaEvent {
            event_type: "message_delta".to_string(),
            message: Some(DeltaMessage {
                id: "msg_mock123".to_string(),
                model: "claude-3-sonnet-20240229".to_string(),
                content: None,
                stop_reason: Some("end_turn".to_string()),
                stop_sequence: None,
                role: Some(Role::Assistant),
                type_field: Some("message".to_string()),
            }),
            delta: Some(Delta {
                text: None,
                stop_reason: Some("end_turn".to_string()),
                stop_sequence: None,
            }),
            usage: Some(Usage {
                input_tokens: 10,
                output_tokens: text_chunks.iter().map(|s| s.len() as u32).sum(),
            }),
            index: Some(text_chunks.len() as u32 + 1),
        });
    }
    
    events
}

/// Setup helper for creating a mock client with a JSON response
#[allow(dead_code)]
pub async fn setup_mock_with_json_response(
    json_text: &str
) -> (Arc<Claude>, Arc<MockApiClient>) {
    let mock_api = Arc::new(MockApiClient::new());
    let response = create_json_response(json_text);
    
    mock_api.add_mock(ClaudeModel::Sonnet, response);
    
    let client = Arc::new(Claude::with_mock_api(
        "test-api-key",
        mock_api_to_handler(mock_api.clone())
    ).with_model(ClaudeModel::Sonnet));
    
    (client, mock_api)
}

/// Setup helper for creating a mock client with streaming text
#[allow(dead_code)]
pub async fn setup_mock_with_streaming_text(
    text_chunks: Vec<&str>
) -> (Arc<Claude>, Arc<MockApiClient>) {
    let mock_api = Arc::new(MockApiClient::new());
    let stream_events = create_mock_stream_response(text_chunks, true);
    
    mock_api.add_stream_response(ClaudeModel::Sonnet, stream_events);
    
    let client = Arc::new(Claude::with_mock_api(
        "test-api-key",
        mock_api_to_handler(mock_api.clone())
    ).with_model(ClaudeModel::Sonnet));
    
    (client, mock_api)
}

// Create a test helper for sentiment analysis
#[allow(dead_code)]
pub fn test_sentiment() -> DomainTester<SentimentAnalysisClient> {
    let mock_api = Arc::new(MockApiClient::new());
    
    // Create a client that uses our mock API handler
    let client = Arc::new(Claude::with_mock_api(
        "test-key", 
        mock_api_to_handler(mock_api.clone())
    ).with_model(ClaudeModel::Sonnet));
    
    let sentiment_client = client.sentiment();
    
    DomainTester::new(sentiment_client, client, mock_api)
}

// Create a test helper for entity extraction
#[allow(dead_code)]
pub fn test_entity() -> DomainTester<EntityExtractionClient> {
    let mock_api = Arc::new(MockApiClient::new());
    
    // Create a client that uses our mock API handler
    let client = Arc::new(Claude::with_mock_api(
        "test-key", 
        mock_api_to_handler(mock_api.clone())
    ).with_model(ClaudeModel::Sonnet));
    
    let entity_client = client.entity();
    
    DomainTester::new(entity_client, client, mock_api)
}

// Create a test helper for content generation
#[allow(dead_code)]
pub fn test_content() -> DomainTester<ContentGenerationClient> {
    let mock_api = Arc::new(MockApiClient::new());
    
    // Create a client that uses our mock API handler
    let client = Arc::new(Claude::with_mock_api(
        "test-key", 
        mock_api_to_handler(mock_api.clone())
    ).with_model(ClaudeModel::Sonnet));
    
    let content_client = client.content();
    
    DomainTester::new(content_client, client, mock_api)
}

// Create a test helper for code assistance
#[allow(dead_code)]
pub fn test_code() -> DomainTester<CodeAssistanceClient> {
    let mock_api = Arc::new(MockApiClient::new());
    
    // Create a client that uses our mock API handler
    let client = Arc::new(Claude::with_mock_api(
        "test-key", 
        mock_api_to_handler(mock_api.clone())
    ).with_model(ClaudeModel::Sonnet));
    
    let code_client = client.code();
    
    DomainTester::new(code_client, client, mock_api)
}

// Helper to create a sentiment analysis response
#[allow(dead_code)]
pub fn create_sentiment_response(sentiment: &str, score: f64) -> MessageResponse {
    let json = format!(r#"```json
{{
    "sentiment": "{}",
    "score": {},
    "aspects": {{}}
}}
```"#, sentiment, score);
    
    create_json_response(&json)
}

// Helper to create an entity extraction response
#[allow(dead_code)]
pub fn create_entity_response(entities: Vec<(&str, &str)>) -> MessageResponse {
    let entities_json: Vec<String> = entities.iter()
        .map(|(text, entity_type)| {
            format!(r#"{{
                "text": "{}",
                "entity_type": "{}",
                "confidence": 0.95,
                "start_idx": 0,
                "end_idx": {},
                "metadata": null
            }}"#, text, entity_type, text.len())
        })
        .collect();
    
    let json = format!(r#"```json
[{}]
```"#, entities_json.join(","));
    
    create_json_response(&json)
}

// Helper to create any JSON response
#[allow(dead_code)]
pub fn create_json_response(json_content: &str) -> MessageResponse {
    MessageResponse {
        id: "msg_mock123".to_string(),
        model: "claude-3-sonnet-20240229".to_string(),
        r#type: "message".to_string(),
        role: Role::Assistant,
        content: vec![Content::Text { 
            text: json_content.to_string() 
        }],
        usage: Usage {
            input_tokens: 10,
            output_tokens: 5,
        },
        stop_reason: Some("end_turn".to_string()),
        stop_sequence: None,
    }
}

// Helper to create plain text response
#[allow(dead_code)]
pub fn create_text_response(text: &str) -> MessageResponse {
    MessageResponse {
        id: "msg_mock123".to_string(),
        model: "claude-3-sonnet-20240229".to_string(),
        r#type: "message".to_string(),
        role: Role::Assistant,
        content: vec![Content::Text { 
            text: text.to_string() 
        }],
        usage: Usage {
            input_tokens: 10,
            output_tokens: 5,
        },
        stop_reason: Some("end_turn".to_string()),
        stop_sequence: None,
    }
}

// Helper to create a code analysis response
#[allow(dead_code)]
pub fn create_code_analysis_response(issues: Vec<(&str, &str)>, score: u32) -> MessageResponse {
    let issues_json: Vec<String> = issues.iter()
        .enumerate()
        .map(|(i, (description, severity))| {
            format!(r#"{{
                "line": {},
                "description": "{}",
                "severity": "{}",
                "code": "let x = 5;"
            }}"#, i + 1, description, severity)
        })
        .collect();
    
    let json = format!(r#"```json
{{
    "issues": [{}],
    "suggestions": [],
    "complexity_score": {},
    "summary": "Code analysis summary"
}}
```"#, issues_json.join(","), score);
    
    create_json_response(&json)
}