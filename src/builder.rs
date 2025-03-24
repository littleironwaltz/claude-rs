// Message Builder

use crate::types::*;
use crate::client::Claude;
use crate::middleware::{ContextManager, RequestMiddleware, ResponseMiddleware};
use crate::utils::{validate_range, StringValidator};

use reqwest::Client as HttpClient;
use std::sync::Arc;
use std::time::Duration;
use futures::StreamExt;

/// A struct for building Claude message requests with a fluent interface.
pub struct MessageBuilder {
    // Version 1: Individual components (backwards compatible)
    http_client: Option<HttpClient>,
    api_key: Option<SecureApiKey>,
    base_url: Option<String>,
    
    // Version 2: Arc'd client reference (more efficient)
    client_ref: Option<Arc<Claude>>,
    
    // Common request parameters
    model: ClaudeModel,
    system: Option<String>,
    messages: Vec<Message>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    tools: Vec<Tool>,
    top_p: Option<f32>,
    top_k: Option<u32>,
    stop_sequences: Vec<String>,
    
    // Middleware components
    context_manager: Option<Arc<dyn ContextManager>>,
    request_middleware: Vec<Arc<dyn RequestMiddleware>>,
    response_middleware: Vec<Arc<dyn ResponseMiddleware>>,
}

impl MessageBuilder {
    /// Create a message builder from individual components
    /// 
    /// This method is kept for backward compatibility.
    #[allow(dead_code)]
    pub(crate) fn new(
        http_client: HttpClient,
        api_key: SecureApiKey,
        base_url: String,
        model: ClaudeModel,
        context_manager: Option<Arc<dyn ContextManager>>,
        request_middleware: Vec<Arc<dyn RequestMiddleware>>,
        response_middleware: Vec<Arc<dyn ResponseMiddleware>>,
    ) -> Self {
        Self {
            http_client: Some(http_client),
            api_key: Some(api_key),
            base_url: Some(base_url),
            client_ref: None,
            model,
            system: None,
            messages: Vec::new(),
            temperature: None,
            max_tokens: None,
            tools: Vec::new(),
            top_p: None,
            top_k: None,
            stop_sequences: Vec::new(),
            context_manager,
            request_middleware,
            response_middleware,
        }
    }
    
    /// Create a message builder from a client reference
    ///
    /// This is the more efficient way to create a message builder as it avoids
    /// cloning most data.
    pub(crate) fn from_client(client: Arc<Claude>) -> Self {
        Self {
            http_client: None,
            api_key: None,
            base_url: None,
            client_ref: Some(client.clone()),
            model: client.default_model.clone(), // Use client's default model
            system: None,
            messages: Vec::new(),
            temperature: None,
            max_tokens: client.default_max_tokens, // Get client default max_tokens if available
            tools: Vec::new(),
            top_p: None,
            top_k: None,
            stop_sequences: Vec::new(),
            context_manager: None, // Will be retrieved from client as needed
            request_middleware: Vec::new(), // Will be retrieved from client as needed
            response_middleware: Vec::new(), // Will be retrieved from client as needed
        }
    }
    
    // Helper methods to get components from either the client_ref or direct fields
    
    /// Get the HTTP client to use for requests
    fn get_http_client(&self) -> &HttpClient {
        if let Some(client) = &self.client_ref {
            &client.http_client
        } else if let Some(http_client) = &self.http_client {
            http_client
        } else {
            panic!("No HTTP client available")
        }
    }
    
    /// Get the API key to use for requests
    fn get_api_key(&self) -> &SecureApiKey {
        if let Some(client) = &self.client_ref {
            &client.api_key
        } else if let Some(api_key) = &self.api_key {
            api_key
        } else {
            panic!("No API key available")
        }
    }
    
    /// Get the base URL to use for requests
    fn get_base_url(&self) -> &str {
        if let Some(client) = &self.client_ref {
            &client.base_url
        } else if let Some(base_url) = &self.base_url {
            base_url
        } else {
            panic!("No base URL available")
        }
    }
    
    /// Get the context manager to use (if any)
    fn get_context_manager(&self) -> Option<Arc<dyn ContextManager>> {
        if self.context_manager.is_some() {
            return self.context_manager.clone();
        }
        
        if let Some(client) = &self.client_ref {
            client.context_manager.clone()
        } else {
            None
        }
    }
    
    /// Get the request middleware to use
    fn get_request_middleware(&self) -> Vec<Arc<dyn RequestMiddleware>> {
        if !self.request_middleware.is_empty() {
            return self.request_middleware.clone();
        }
        
        if let Some(client) = &self.client_ref {
            client.request_middleware.clone()
        } else {
            Vec::new()
        }
    }
    
    /// Get the response middleware to use
    fn get_response_middleware(&self) -> Vec<Arc<dyn ResponseMiddleware>> {
        if !self.response_middleware.is_empty() {
            return self.response_middleware.clone();
        }
        
        if let Some(client) = &self.client_ref {
            client.response_middleware.clone()
        } else {
            Vec::new()
        }
    }
    
    /// Set the system prompt for the message
    ///
    /// The system prompt provides high-level instructions for the assistant.
    pub fn system(mut self, system: impl Into<String>) -> ClaudeResult<Self> {
        self.system = Some(StringValidator::not_empty(system, "system")?);
        Ok(self)
    }
    
    /// Set the model to use for the message
    ///
    /// Overrides the default model from the client.
    pub fn model(mut self, model: ClaudeModel) -> Self {
        self.model = model;
        self
    }
    
    /// Add a user message with text content
    ///
    /// This is a convenience method for adding user messages.
    pub fn user_content(mut self, text: impl Into<String>) -> Self {
        let message = Message {
            role: Role::User,
            content: vec![Content::Text { text: text.into() }],
        };
        self.messages.push(message);
        self
    }
    
    /// Alias for user_content for backward compatibility
    pub fn user_message(self, text: impl Into<String>) -> ClaudeResult<Self> {
        let text_str = text.into();
        if text_str.trim().is_empty() {
            return Err(ClaudeError::ValidationError("User message cannot be empty".into()));
        }
        Ok(self.user_content(text_str))
    }
    
    /// Add an assistant message with text content
    ///
    /// This is a convenience method for adding assistant messages.
    pub fn assistant_content(mut self, text: impl Into<String>) -> Self {
        let message = Message {
            role: Role::Assistant,
            content: vec![Content::Text { text: text.into() }],
        };
        self.messages.push(message);
        self
    }
    
    /// Alias for assistant_content for backward compatibility
    pub fn assistant_message(self, text: impl Into<String>) -> Self {
        self.assistant_content(text)
    }
    
    /// Add a raw message directly
    ///
    /// This allows adding messages with more complex content types.
    pub fn add_message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }
    
    /// Adds a message with a custom content type (alias for backward compatibility)
    pub fn add_raw_message(self, message: Message) -> Self {
        self.add_message(message)
    }
    
    /// Set the temperature parameter (between 0.0 and 1.0)
    ///
    /// Controls randomness in the response. Lower values are more deterministic,
    /// higher values more creative.
    pub fn temperature(mut self, temperature: f32) -> ClaudeResult<Self> {
        self.temperature = Some(validate_range(temperature, 0.0, 1.0, "temperature")?);
        Ok(self)
    }
    
    /// Set the maximum number of tokens to generate
    ///
    /// Limits the length of the response.
    pub fn max_tokens(mut self, max_tokens: u32) -> ClaudeResult<Self> {
        if max_tokens == 0 {
            return Err(ClaudeError::ValidationError("max_tokens must be greater than 0".into()));
        }
        self.max_tokens = Some(max_tokens);
        Ok(self)
    }
    
    /// Add a tool definition that Claude can use
    ///
    /// Tools allow Claude to call external functions.
    pub fn add_tool(mut self, tool: Tool) -> Self {
        self.tools.push(tool);
        self
    }
    
    /// Set the top_p parameter (between 0.0 and 1.0)
    ///
    /// Controls nucleus sampling - only consider tokens whose cumulative probability
    /// exceeds this threshold.
    pub fn top_p(mut self, top_p: f32) -> ClaudeResult<Self> {
        self.top_p = Some(validate_range(top_p, 0.0, 1.0, "top_p")?);
        Ok(self)
    }
    
    /// Set the top_k parameter (positive integer)
    ///
    /// Limits the number of token choices considered at each step - only the k most
    /// likely tokens are selected.
    pub fn top_k(mut self, top_k: u32) -> ClaudeResult<Self> {
        if top_k == 0 {
            return Err(ClaudeError::ValidationError("top_k must be greater than 0".into()));
        }
        self.top_k = Some(top_k);
        Ok(self)
    }
    
    /// Add a stop sequence (non-empty string)
    ///
    /// Defines sequences that will cause the model to stop generating. 
    /// Useful for controlling output format.
    pub fn add_stop_sequence(mut self, sequence: impl Into<String>) -> ClaudeResult<Self> {
        let sequence = StringValidator::not_empty(sequence, "stop sequence")?;
        
        // Enforce the maximum number of stop sequences
        if self.stop_sequences.len() >= 256 {
            return Err(ClaudeError::ValidationError(
                "Cannot add more than 256 stop sequences".to_string()
            ));
        }
        
        self.stop_sequences.push(sequence);
        Ok(self)
    }
    
    /// Prepare a request for sending to the Claude API
    ///
    /// This validates parameters, applies middleware, and formats the request appropriately.
    async fn prepare_request(&self, streaming: bool) -> ClaudeResult<(String, MessageRequest)> {
        // Get processed messages from context manager (if available)
        let processed_messages = match self.get_context_manager() {
            Some(context_manager) => context_manager.process_messages(self.messages.clone()).await?,
            None => self.messages.clone(),
        };
        
        // Construct the API endpoint
        let endpoint = format!("{}/messages", self.get_base_url());
        
        // Create the request body
        let mut request = MessageRequest {
            model: self.model.as_str().to_string(),
            messages: processed_messages,
            system: self.system.clone(),
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            tools: if self.tools.is_empty() { None } else { Some(self.tools.clone()) },
            top_p: self.top_p,
            top_k: self.top_k,
            stop_sequences: self.stop_sequences.clone(),
            stream: if streaming { Some(true) } else { None },
        };
        
        // Apply request middleware (if any)
        for middleware in self.get_request_middleware() {
            request = middleware.process_request(request).await?;
        }
        
        Ok((endpoint, request))
    }
    
    /// Handle error responses from the Claude API
    ///
    /// This method checks for error status codes and formats appropriate error messages.
    async fn handle_error_response(&self, response: reqwest::Response) -> ClaudeResult<reqwest::Response> {
        if response.status().is_success() {
            return Ok(response);
        }
        
        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        
        if status == 429 {
            // Rate limit handling
            let retry_after = headers
                .get("retry-after")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .map(Duration::from_secs);
                
            return Err(ClaudeError::rate_limited(retry_after));
        }
        
        // Sanitize error message before returning
        let sanitized_error = sanitize_error_message(&error_text);
        
        Err(ClaudeError::api_error(sanitized_error, Some(status), None, Some(concat!(file!(), ":", line!()))))
    }
    
    /// Send the message and get a response
    ///
    /// This method validates parameters and sends the request to the Claude API.
    /// It handles parameter validation, middleware processing, and context management.
    pub async fn send(self) -> ClaudeResult<MessageResponse> {
        // Validate minimum requirements
        if self.messages.is_empty() {
            return Err(ClaudeError::ValidationError(
                "At least one message is required".to_string()
            ));
        }
        
        // Prepare the request
        let (endpoint, request) = self.prepare_request(false).await?;
        
        // Handle the actual sending of the request - could be real or mock
        let mut message_response = self.execute_request(&endpoint, request.clone()).await?;
        
        // Apply response middleware (if any)
        let middlewares = self.get_response_middleware();
        for middleware in middlewares {
            message_response = middleware.process_response(message_response).await?;
        }
        
        // Update context manager with the response (if available)
        if let Some(context_manager) = self.get_context_manager() {
            context_manager.update_with_response(&message_response).await?;
        }
        
        Ok(message_response)
    }
    
    /// Execute a request, potentially using a mock handler if one is available
    async fn execute_request(&self, endpoint: &str, request: MessageRequest) -> ClaudeResult<MessageResponse> {
        // First, check if we have a custom request handler from a mock
        if let Some(client) = &self.client_ref {
            // Get the handler outside of the await
            let handler_opt = {
                if let Ok(guard) = client.request_handler.lock() {
                    // Clone the handler if it exists
                    (*guard).as_ref().map(|handler| handler.clone())
                } else {
                    None
                }
            };
            
            // Use the handler if we got one
            if let Some(handler) = handler_opt {
                // Use the custom handler
                return handler(request.clone()).await;
            }
        }
        
        // If we reach here, use the regular HTTP client
        let response = self.get_http_client()
            .post(endpoint)
            .header("x-api-key", self.get_api_key().as_str())
            .header("anthropic-version", "2023-06-01")
            .json(&request)
            .send()
            .await?;
            
        // Check for errors
        let response = self.handle_error_response(response).await?;
        
        // Parse the response
        response.json::<MessageResponse>().await.map_err(|e| ClaudeError::parse_error(
            e.to_string(), 
            None, 
            Some(e), 
            Some(concat!(file!(), ":", line!()))
        ))
    }
    
    /// Send the message and get a streaming response
    ///
    /// This method is similar to send(), but returns a stream of delta events
    /// that can be processed incrementally as they arrive.
    pub async fn stream(self) -> ClaudeResult<MessageStream> {
        // Validate minimum requirements
        if self.messages.is_empty() {
            return Err(ClaudeError::ValidationError(
                "At least one message is required".to_string()
            ));
        }
        
        // Prepare the request
        let (endpoint, request) = self.prepare_request(true).await?;
        
        // Handle the streaming request - this could be real or mock
        self.execute_stream_request(&endpoint, request.clone()).await
    }
    
    /// Execute a streaming request, potentially using a mock handler if one is available
    async fn execute_stream_request(&self, endpoint: &str, request: MessageRequest) -> ClaudeResult<MessageStream> {
        // First, check if we have a custom stream handler from a mock
        if let Some(client) = &self.client_ref {
            // Get the handler outside of the await
            let handler_opt = {
                if let Ok(guard) = client.stream_handler.lock() {
                    // Clone the handler if it exists
                    (*guard).as_ref().map(|handler| handler.clone())
                } else {
                    None
                }
            };
            
            // Use the handler if we got one
            if let Some(handler) = handler_opt {
                // Use the custom handler
                return handler(request.clone()).await;
            }
        }
        
        // Ensure stream parameter is set
        let mut streaming_request = request.clone();
        streaming_request.stream = Some(true);
        
        // Send the HTTP request
        let response = self.get_http_client()
            .post(endpoint)
            .header("x-api-key", self.get_api_key().as_str())
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .header("accept", "text/event-stream")  // Explicitly request SSE format
            .json(&streaming_request)
            .send()
            .await?;
            
        // Check for errors
        let response = self.handle_error_response(response).await?;
        
        // Process the stream
        let stream = response.bytes_stream();
        
        // Transform the bytes stream into a message stream
        let message_stream = stream
            .map(|result| {
                result.map_err(|e| ClaudeError::request_error(
                    e.to_string(), 
                    None, 
                    Some(e), 
                    Some(concat!(file!(), ":", line!()))
                ))
            })
            .map(|result| {
                result.and_then(|bytes: bytes::Bytes| {
                    // SSE event parsing (format: data: {...}\n\n)
                    let text = String::from_utf8_lossy(&bytes);
                    let lines: Vec<&str> = text.lines().collect();
                    
                    let mut events = Vec::new();
                    let mut current_data = String::new();
                    
                    for line in lines {
                        if line.is_empty() && !current_data.is_empty() {
                            // Empty line triggers parsing of existing data
                            if current_data == "[DONE]" {
                                // Stream end marker
                                continue;
                            }
                            
                            match serde_json::from_str::<DeltaEvent>(&current_data) {
                                Ok(event) => events.push(event),
                                Err(e) => return Err(ClaudeError::parse_error(
                                    format!("Failed to parse event: {}", e),
                                    Some(current_data.clone()),
                                    Some(e),
                                    Some(concat!(file!(), ":", line!()))
                                )),
                            }
                            current_data.clear();
                        } else if let Some(data) = line.strip_prefix("data: ") {
                            current_data = data.to_string();
                        }
                    }
                    
                    // Process any remaining data
                    if !current_data.is_empty() && current_data != "[DONE]" {
                        match serde_json::from_str::<DeltaEvent>(&current_data) {
                            Ok(event) => events.push(event),
                            Err(e) => return Err(ClaudeError::parse_error(
                                format!("Failed to parse final event: {}", e),
                                Some(current_data),
                                Some(e),
                                Some(concat!(file!(), ":", line!()))
                            )),
                        }
                    }
                    
                    Ok(events)
                })
            })
            .flat_map(|result| -> futures::stream::BoxStream<'static, Result<DeltaEvent, ClaudeError>> {
                match result {
                    Ok(events) => futures::stream::iter(events.into_iter().map(Ok)).boxed(),
                    Err(e) => futures::stream::iter(vec![Err(e)]).boxed(),
                }
            })
            .boxed();
        
        Ok(message_stream)
    }
}