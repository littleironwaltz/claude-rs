use claude_rs::types::*;
use claude_rs::client::MockApiHandler;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::pin::Pin;
use tokio::sync::mpsc;
use std::future::Future;
use std::time::Duration;

/// Common response type for both regular and streaming responses
#[derive(Clone)]
#[allow(dead_code)]
pub enum MockResponse {
    Success(MessageResponse),
    Error(ClaudeError),
    StreamSuccess(Vec<DeltaEvent>),
    StreamError(ClaudeError),
}

// Implement From traits for converting types to MockResponse
impl From<MessageResponse> for MockResponse {
    fn from(resp: MessageResponse) -> Self {
        MockResponse::Success(resp)
    }
}

impl From<Vec<DeltaEvent>> for MockResponse {
    fn from(events: Vec<DeltaEvent>) -> Self {
        MockResponse::StreamSuccess(events)
    }
}

impl From<(ClaudeError, bool)> for MockResponse {
    fn from((error, is_stream): (ClaudeError, bool)) -> Self {
        if is_stream {
            MockResponse::StreamError(error)
        } else {
            MockResponse::Error(error)
        }
    }
}

impl MockResponse {
    // Helper to convert to regular response result
    pub fn to_regular_result(&self) -> ClaudeResult<MessageResponse> {
        match self {
            MockResponse::Success(resp) => Ok(resp.clone()),
            MockResponse::Error(err) => Err(err.clone()),
            _ => Err(ClaudeError::request_error(
                "Cannot convert streaming response to regular response", 
                None,
                None::<reqwest::Error>,
                Some(concat!(file!(), ":", line!()))
            )),
        }
    }
    
    // Helper to convert to streaming response result
    pub fn to_stream_result(&self) -> ClaudeResult<MessageStream> {
        match self {
            MockResponse::StreamSuccess(events) => {
                let events = events.clone();
                let (tx, rx) = mpsc::channel(10);
                
                tokio::spawn(async move {
                    for event in events {
                        // Simulate realistic streaming with small delays
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        
                        // Send the event through the channel
                        if tx.send(Ok(event)).await.is_err() {
                            break;
                        }
                    }
                });
                
                // Create a basic stream from the receiver channel
                let stream = futures::stream::unfold(rx, |mut rx| async move {
                    rx.recv().await.map(|item| (item, rx))
                });
                
                Ok(Box::pin(stream))
            },
            MockResponse::StreamError(err) => Err(err.clone()),
            _ => Err(ClaudeError::request_error(
                "Cannot convert regular response to streaming response", 
                None,
                None::<reqwest::Error>,
                Some(concat!(file!(), ":", line!()))
            )),
        }
    }
}

/// Mock API client for testing purposes
#[derive(Clone)]
pub struct MockApiClient {
    inner: Arc<Mutex<MockApiClientInner>>,
}

struct MockApiClientInner {
    // Responses for different models
    responses: HashMap<String, MockResponse>,
    // Request history for verification
    request_history: Vec<MessageRequest>,
    // Delay simulation (if needed)
    response_delay: Option<Duration>,
}

#[allow(dead_code)]
impl Default for MockApiClient {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(MockApiClientInner {
                responses: HashMap::new(),
                request_history: Vec::new(),
                response_delay: None,
            })),
        }
    }
}

#[allow(dead_code)]
impl MockApiClient {
    /// Create a new mock API client
    pub fn new() -> Self {
        Self::default()
    }

    // Generic method to add a response for a model
    fn add_model_response(&self, model: ClaudeModel, response: MockResponse) -> &Self {
        let model_str = model.as_str().to_string();
        let mut inner = self.inner.lock().unwrap();
        inner.responses.insert(model_str, response);
        self
    }

    /// Add a predefined mock response for a specific model
    /// Accepts any type that can be converted to MockResponse
    pub fn add_mock<T: Into<MockResponse>>(&self, model: ClaudeModel, response: T) -> &Self {
        let model_str = model.as_str().to_string();
        let mut inner = self.inner.lock().unwrap();
        inner.responses.insert(model_str, response.into());
        self
    }

    /// Add a predefined response for a specific model (legacy method)
    pub fn add_response(
        &self,
        model: ClaudeModel,
        response: MessageResponse,
    ) -> &Self {
        self.add_mock(model, response)
    }

    /// Add a predefined streaming response for a specific model (legacy method)
    pub fn add_stream_response(
        &self,
        model: ClaudeModel,
        delta_events: Vec<DeltaEvent>,
    ) -> &Self {
        self.add_mock(model, delta_events)
    }

    /// Add a predefined error response for a specific model (legacy method)
    pub fn add_error(
        &self,
        model: ClaudeModel,
        error: ClaudeError,
    ) -> &Self {
        self.add_mock(model, (error, false))
    }

    /// Add a predefined streaming error response for a specific model (legacy method)
    pub fn add_stream_error(
        &self,
        model: ClaudeModel,
        error: ClaudeError,
    ) -> &Self {
        self.add_mock(model, (error, true))
    }

    /// Set a simulated response delay
    pub fn with_delay(&self, delay: Duration) -> &Self {
        let mut inner = self.inner.lock().unwrap();
        inner.response_delay = Some(delay);
        self
    }

    /// Get the captured request history
    pub fn get_request_history(&self) -> Vec<MessageRequest> {
        let inner = self.inner.lock().unwrap();
        inner.request_history.clone()
    }

    /// Clear the request history
    pub fn clear_request_history(&self) -> &Self {
        let mut inner = self.inner.lock().unwrap();
        inner.request_history.clear();
        self
    }
    
    // Record a request for later inspection
    fn record_request(&self, request: MessageRequest) {
        let mut inner = self.inner.lock().unwrap();
        inner.request_history.push(request);
    }
    
    // Process delay if configured (legacy method - use simulate_delay instead)
    async fn process_delay(&self) {
        self.simulate_delay().await;
    }

    /// Process a request and return a response based on configured mock behavior
    /// 
    /// This method:
    /// 1. Records the request for later inspection
    /// 2. Simulates any configured delay
    /// 3. Looks up the appropriate mock response for the model
    /// 4. Returns the response or error
    pub async fn process_request(&self, request: MessageRequest) -> ClaudeResult<MessageResponse> {
        // Record request for test verification
        self.record_request(request.clone());
        
        // Apply configured delay (if any)
        self.simulate_delay().await;
        
        // Get configured response for this model
        let model = &request.model;
        let response = self.get_response_for_model(model)?;
        
        // Convert to appropriate response type
        response.to_regular_result()
    }

    /// Simulates network delay if configured
    async fn simulate_delay(&self) {
        if let Some(delay) = self.get_configured_delay() {
            tokio::time::sleep(delay).await;
        }
    }

    /// Retrieves configured delay (if any)
    fn get_configured_delay(&self) -> Option<Duration> {
        let inner = self.inner.lock().unwrap();
        inner.response_delay
    }

    /// Retrieves configured response for specified model
    fn get_response_for_model(&self, model: &str) -> ClaudeResult<MockResponse> {
        let inner = self.inner.lock().unwrap();
        match inner.responses.get(model) {
            Some(response) => Ok(response.clone()),
            None => Err(ClaudeError::api_error(
                format!("No mock response configured for model: {}", model),
                Some(404),
                None,
                Some(concat!(file!(), ":", line!()))
            )),
        }
    }
    
    /// Debug method to get the response for a specific model
    pub fn debug_get_response_for_model(&self, model: ClaudeModel) -> Option<String> {
        let inner = self.inner.lock().unwrap();
        let model_str = model.as_str().to_string();
        
        if let Some(response) = inner.responses.get(&model_str) {
            match response {
                MockResponse::Success(_) => Some("SUCCESS_RESPONSE".to_string()),
                MockResponse::Error(err) => {
                    match err {
                        ClaudeError::ApiError { .. } => Some("API_ERROR".to_string()),
                        ClaudeError::RequestError { .. } => Some("REQUEST_ERROR".to_string()),
                        ClaudeError::ParseError { .. } => Some("PARSE_ERROR".to_string()),
                        ClaudeError::DomainError { .. } => Some("DOMAIN_ERROR".to_string()),
                        _ => Some("OTHER_ERROR".to_string()),
                    }
                },
                MockResponse::StreamSuccess(_) => Some("STREAM_SUCCESS".to_string()),
                MockResponse::StreamError(_) => Some("STREAM_ERROR".to_string()),
            }
        } else {
            None
        }
    }

    /// Process a streaming request and return a stream of DeltaEvents
    /// 
    /// This method:
    /// 1. Records the request for later inspection
    /// 2. Simulates any configured delay
    /// 3. Looks up the appropriate mock stream response for the model
    /// 4. Returns a stream of deltas or an error
    pub async fn process_stream_request(
        &self,
        request: MessageRequest,
    ) -> ClaudeResult<MessageStream> {
        // Record the request for test verification
        self.record_request(request.clone());
        
        // Apply configured delay (if any)
        self.simulate_delay().await;
        
        // Get the response for this model
        let model = &request.model;
        let response = self.get_response_for_model(model)?;
        
        // Convert to the appropriate streaming result
        response.to_stream_result()
    }
}

// Helper function to create a sample message response with text
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
            output_tokens: text.split_whitespace().count() as u32,
        },
        stop_reason: Some("end_turn".to_string()),
        stop_sequence: None,
    }
}

// Helper function to create a sample message response with JSON in a code block
#[allow(dead_code)]
pub fn create_json_response(json: &str) -> MessageResponse {
    create_text_response(&format!("```json\n{}\n```", json))
}

// Helper function to create a basic sample message response
#[allow(dead_code)]
pub fn create_sample_message_response() -> MessageResponse {
    create_text_response("This is a sample response from the mock API client.")
}

// Helper function to create a code block response
#[allow(dead_code)]
pub fn create_code_response(language: &str, code: &str) -> MessageResponse {
    create_text_response(&format!("```{}\n{}\n```", language, code))
}

/// Helper to create a sentiment analysis response
#[allow(dead_code)]
pub fn create_sentiment_response(sentiment: &str, score: f32) -> MessageResponse {
    let json = format!(r#"{{
        "sentiment": "{}",
        "score": {},
        "aspects": {{}}
    }}"#, sentiment, score);
    
    create_json_response(&json)
}

/// Helper to create an entity extraction response
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
    
    let json = format!("[{}]", entities_json.join(","));
    create_json_response(&json)
}

/// Helper to create a code analysis response
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
    
    let json = format!(r#"{{
        "issues": [{}],
        "suggestions": [],
        "complexity_score": {},
        "summary": "Code analysis summary"
    }}"#, issues_json.join(","), score);
    
    create_json_response(&json)
}

// Helper function to create sample delta events for streaming
#[allow(dead_code)]
pub fn create_sample_delta_events() -> Vec<DeltaEvent> {
    vec![
        DeltaEvent {
            event_type: "message_start".to_string(),
            message: Some(DeltaMessage {
                id: "msg_sample123".to_string(),
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
        },
        DeltaEvent {
            event_type: "content_block_delta".to_string(),
            message: Some(DeltaMessage {
                id: "msg_sample123".to_string(),
                model: "claude-3-sonnet-20240229".to_string(),
                content: Some(vec![Content::Text {
                    text: "This is ".to_string(),
                }]),
                stop_reason: None,
                stop_sequence: None,
                role: Some(Role::Assistant),
                type_field: Some("message".to_string()),
            }),
            delta: Some(Delta {
                text: Some("This is ".to_string()),
                stop_reason: None,
                stop_sequence: None,
            }),
            usage: None,
            index: Some(1),
        },
        DeltaEvent {
            event_type: "content_block_delta".to_string(),
            message: Some(DeltaMessage {
                id: "msg_sample123".to_string(),
                model: "claude-3-sonnet-20240229".to_string(),
                content: Some(vec![Content::Text {
                    text: "a sample ".to_string(),
                }]),
                stop_reason: None,
                stop_sequence: None,
                role: Some(Role::Assistant),
                type_field: Some("message".to_string()),
            }),
            delta: Some(Delta {
                text: Some("a sample ".to_string()),
                stop_reason: None,
                stop_sequence: None,
            }),
            usage: None,
            index: Some(2),
        },
        DeltaEvent {
            event_type: "content_block_delta".to_string(),
            message: Some(DeltaMessage {
                id: "msg_sample123".to_string(),
                model: "claude-3-sonnet-20240229".to_string(),
                content: Some(vec![Content::Text {
                    text: "streaming response".to_string(),
                }]),
                stop_reason: None,
                stop_sequence: None,
                role: Some(Role::Assistant),
                type_field: Some("message".to_string()),
            }),
            delta: Some(Delta {
                text: Some("streaming response".to_string()),
                stop_reason: None,
                stop_sequence: None,
            }),
            usage: None,
            index: Some(3),
        },
        DeltaEvent {
            event_type: "content_block_delta".to_string(),
            message: Some(DeltaMessage {
                id: "msg_sample123".to_string(),
                model: "claude-3-sonnet-20240229".to_string(),
                content: Some(vec![Content::Text {
                    text: " from the mock API.".to_string(),
                }]),
                stop_reason: None,
                stop_sequence: None,
                role: Some(Role::Assistant),
                type_field: Some("message".to_string()),
            }),
            delta: Some(Delta {
                text: Some(" from the mock API.".to_string()),
                stop_reason: None,
                stop_sequence: None,
            }),
            usage: None,
            index: Some(4),
        },
        DeltaEvent {
            event_type: "message_delta".to_string(),
            message: Some(DeltaMessage {
                id: "msg_sample123".to_string(),
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
                input_tokens: 15,
                output_tokens: 12,
            }),
            index: Some(5),
        },
    ]
}

// Implement the MockApiHandler trait for MockApiClient
impl MockApiHandler for MockApiClient {
    fn process_request(&self, request: MessageRequest) -> Pin<Box<dyn Future<Output = ClaudeResult<MessageResponse>> + Send>> {
        let this = self.clone();
        Box::pin(async move {
            this.process_request(request).await
        })
    }
    
    fn process_stream_request(&self, request: MessageRequest) -> Pin<Box<dyn Future<Output = ClaudeResult<MessageStream>> + Send>> {
        let this = self.clone();
        Box::pin(async move {
            this.process_stream_request(request).await
        })
    }
}

impl MockApiClient {
    /// Convert this MockApiClient to a handler trait object for use with Claude client
    pub fn as_handler(self: Arc<Self>) -> Arc<dyn MockApiHandler> {
        self
    }
    
    /// Configure mock with deterministic timing for stable tests
    #[allow(dead_code)]
    pub fn with_deterministic_timing(self) -> Self {
        // Set a predictable timing for tests (10ms is enough to simulate async but not slow tests)
        self.with_delay(Duration::from_millis(10));
        self
    }
}

/// Convert Arc<MockApiClient> to Arc<dyn MockApiHandler>
#[allow(dead_code)]
pub fn mock_api_to_handler(mock: Arc<MockApiClient>) -> Arc<dyn MockApiHandler> {
    mock.as_handler()
}