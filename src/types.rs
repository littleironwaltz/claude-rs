// Core types and errors

use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::time::Duration;
use std::pin::Pin;
use futures::Stream;
use std::fmt;
use std::ops::Deref;
use std::future::Future;
use std::sync::Arc;

/// The result type used throughout the Claude SDK
pub type ClaudeResult<T> = Result<T, ClaudeError>;

/// Convert reqwest::Error to our ClaudeError
impl From<reqwest::Error> for ClaudeError {
    fn from(err: reqwest::Error) -> Self {
        ClaudeError::RequestError { 
            message: err.to_string(),
            details: None,
            location: None,
            source: Some(Arc::new(err) as Arc<dyn std::error::Error + Send + Sync>),
        }
    }
}

/// Type alias for message streams to simplify function signatures
pub type MessageStream = Pin<Box<dyn Stream<Item = Result<DeltaEvent, ClaudeError>> + Send>>;

/// Type alias for future returning JSON
pub type JsonFuture<'a, T> = Pin<Box<dyn Future<Output = ClaudeResult<T>> + Send + 'a>>;

/// Type alias for future returning text
pub type TextFuture<'a> = Pin<Box<dyn Future<Output = ClaudeResult<String>> + Send + 'a>>;

/// A secure container for API keys that automatically zeroes memory when dropped
pub struct SecureApiKey {
    key: String,
}

impl SecureApiKey {
    /// Create a new secure API key
    pub fn new(key: impl Into<String>) -> Self {
        let key = key.into();
        // Basic validation could be added here
        Self { key }
    }

    /// Get a reference to the underlying key
    pub fn as_str(&self) -> &str {
        &self.key
    }
}

// Implement Deref for convenience in passing to reqwest headers
impl Deref for SecureApiKey {
    type Target = str;
    
    fn deref(&self) -> &Self::Target {
        &self.key
    }
}

// Implement Drop to zero memory when the key is dropped
impl Drop for SecureApiKey {
    fn drop(&mut self) {
        // Overwrite the string with zeros to remove sensitive data from memory
        unsafe {
            let bytes = self.key.as_bytes_mut();
            bytes.iter_mut().for_each(|b| *b = 0);
        }
    }
}

// Prevent accidental printing of API keys in logs/debug output
impl fmt::Debug for SecureApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecureApiKey([REDACTED])")
    }
}

// Display implementation also redacts the key
impl fmt::Display for SecureApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED API KEY]")
    }
}

// Clone implementation for SecureApiKey
impl Clone for SecureApiKey {
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
        }
    }
}

#[derive(Debug, Error, Clone)]
pub enum ClaudeError {
    #[error("API request failed: {message}")]
    RequestError {
        message: String,
        details: Option<String>,
        location: Option<String>,  // New field for call location
        source: Option<Arc<dyn std::error::Error + Send + Sync>>,  // New field for error chain
    },
    
    #[error("Failed to parse API response: {message}")]
    ParseError {
        message: String,
        source_text: Option<String>,
        location: Option<String>,  // New field for call location
        source: Option<Arc<dyn std::error::Error + Send + Sync>>,  // New field for error chain
    },
    
    #[error("Rate limited by API: retry after {retry_after:?}")]
    RateLimited {
        retry_after: Option<Duration>,
        details: Option<String>,
        location: Option<String>,  // New field for call location
    },
    
    #[error("API key not provided")]
    MissingApiKey {
        location: Option<String>,  // New field for call location
    },
    
    #[error("API returned error: {status} - {message}")]
    ApiError { 
        status: u16, 
        message: String,
        response_body: Option<String>,
        location: Option<String>,  // New field for call location
    },
    
    #[error("Context window exceeded")]
    ContextExceeded {
        tokens: Option<u32>,
        max_tokens: Option<u32>,
        location: Option<String>,  // New field for call location
    },
    
    #[error("Invalid model specified: {0}")]
    InvalidModel(String),
    
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
    
    #[error("Domain error: {domain}: {message}")]
    DomainError { 
        domain: String, 
        message: String,
        details: Option<String>,
        location: Option<String>,  // New field for call location
        source: Option<Arc<dyn std::error::Error + Send + Sync>>,  // New field for error chain
    },
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Conversion error: {0}")]
    ConversionError(String),
}

/// Claude model identifiers
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClaudeModel {
    #[serde(rename = "claude-3-opus-20240229")]
    Opus,
    #[serde(rename = "claude-3-sonnet-20240229")]
    Sonnet,
    #[serde(rename = "claude-3-haiku-20240307")]
    Haiku,
    #[serde(rename = "claude-3-5-sonnet-20240620")]
    Sonnet35,
    #[serde(rename = "claude-3-7-sonnet-20250219")]
    Sonnet37,
    /// Use a custom model identifier
    Custom(String),
}

impl ClaudeModel {
    pub fn as_str(&self) -> &str {
        match self {
            ClaudeModel::Opus => "claude-3-opus-20240229",
            ClaudeModel::Sonnet => "claude-3-sonnet-20240229",
            ClaudeModel::Haiku => "claude-3-haiku-20240307",
            ClaudeModel::Sonnet35 => "claude-3-5-sonnet-20240620",
            ClaudeModel::Sonnet37 => "claude-3-7-sonnet-20250219",
            ClaudeModel::Custom(id) => id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<Content>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Content {
    Text { text: String },
    Image { source: ImageSource },
    Tool { tool_use: ToolUse },
    ToolResult { tool_result: ToolResult, tool_call_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub media_type: String,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUse {
    pub id: String,
    pub name: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Serialize, Clone)]
pub struct MessageRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "std::vec::Vec::is_empty")]
    pub stop_sequences: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MessageResponse {
    pub id: String,
    pub model: String,
    pub r#type: String,
    pub role: Role,
    pub content: Vec<Content>,
    pub usage: Usage,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Usage {
    /// Number of input tokens - Optional in streaming final events
    #[serde(default)]
    pub input_tokens: u32,
    /// Number of output tokens - Optional in streaming final events
    #[serde(default)]
    pub output_tokens: u32,
}

// Delta content for streaming response
#[derive(Debug, Deserialize, Clone)]
pub struct DeltaEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub message: Option<DeltaMessage>,
    pub index: Option<u32>,
    pub usage: Option<Usage>,
    // New field
    pub delta: Option<Delta>,
}

impl DeltaEvent {
    /// Extract text from any delta format
    /// 
    /// This method supports multiple formats for backward compatibility:
    /// 1. New format (using the delta.text field)
    /// 2. Old format (using the message.content field)
    /// 3. Content blocks (with text content type)
    ///
    /// Returns None for non-text events like message_start, message_stop, etc.
    pub fn to_text(&self) -> Option<String> {
        // Skip events that don't contain text
        // These event types are control messages, not content
        if self.event_type == "message_start" || 
           self.event_type == "message_stop" ||
           self.event_type == "message_delta" && self.delta.as_ref().and_then(|d| d.text.as_ref()).is_none() {
            return None;
        }
        
        // First try the new delta format (preferred)
        if let Some(delta) = &self.delta {
            if let Some(text) = &delta.text {
                if !text.is_empty() {
                    return Some(text.clone());
                }
            }
        }
        
        // Fall back to the old format
        if let Some(msg) = &self.message {
            if let Some(contents) = &msg.content {
                for content in contents {
                    if let Content::Text { text } = content {
                        if !text.is_empty() {
                            return Some(text.clone());
                        }
                    }
                }
            }
        }
        
        None
    }
    
    /// Check if this is a final event (with stop_reason)
    pub fn is_final(&self) -> bool {
        // Event type-based detection (most reliable)
        if self.event_type == "message_stop" || self.event_type == "message_delta" {
            return true;
        }
        
        // Check in new delta format
        if let Some(delta) = &self.delta {
            if delta.stop_reason.is_some() {
                return true;
            }
        }
        
        // Check in old format
        if let Some(msg) = &self.message {
            if msg.stop_reason.is_some() {
                return true;
            }
        }
        
        // Check usage presence as a fallback (not always reliable)
        self.usage.is_some()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Delta {
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DeltaMessage {
    pub id: String,
    pub model: String,
    pub content: Option<Vec<Content>>,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    // New fields
    pub role: Option<Role>,
    #[serde(rename = "type")]
    pub type_field: Option<String>,
}

// Implementation of helper methods for ClaudeError
impl ClaudeError {
    // Enhanced error helpers with location and source tracking
    pub fn request_error<T: Into<String>>(
        message: T, 
        details: Option<String>,
        source: Option<impl std::error::Error + Send + Sync + 'static>,
        location: Option<&str>
    ) -> Self {
        let error = Self::RequestError {
            message: message.into(),
            details,
            location: location.map(String::from),
            source: source.map(|e| Arc::new(e) as Arc<dyn std::error::Error + Send + Sync>),
        };
        
        // Optional logging integration
        if let Some(loc) = &error.location() {
            log::error!("{} at {}", error, loc);
        } else {
            log::error!("{}", error);
        }
        
        error
    }
    
    pub fn parse_error<T: Into<String>>(
        message: T, 
        source_text: Option<String>,
        source: Option<impl std::error::Error + Send + Sync + 'static>,
        location: Option<&str>
    ) -> Self {
        let error = Self::ParseError {
            message: message.into(),
            source_text,
            location: location.map(String::from),
            source: source.map(|e| Arc::new(e) as Arc<dyn std::error::Error + Send + Sync>),
        };
        
        // Optional logging integration
        if let Some(loc) = &error.location() {
            log::error!("{} at {}", error, loc);
        } else {
            log::error!("{}", error);
        }
        
        error
    }
    
    pub fn domain_error<T: Into<String>>(
        message: T, 
        domain: Option<String>, 
        details: Option<String>,
        source: Option<impl std::error::Error + Send + Sync + 'static>,
        location: Option<&str>
    ) -> Self {
        let error = Self::DomainError {
            message: message.into(),
            domain: domain.unwrap_or_default(),
            details,
            location: location.map(String::from),
            source: source.map(|e| Arc::new(e) as Arc<dyn std::error::Error + Send + Sync>),
        };
        
        // Optional logging integration
        if let Some(loc) = &error.location() {
            log::error!("{} at {}", error, loc);
        } else {
            log::error!("{}", error);
        }
        
        error
    }
    
    pub fn api_error<T: Into<String>>(
        message: T, 
        status: Option<u16>, 
        response_body: Option<String>,
        location: Option<&str>
    ) -> Self {
        let error = Self::ApiError {
            message: message.into(),
            status: status.unwrap_or(500),
            response_body,
            location: location.map(String::from),
        };
        
        // Optional logging integration
        if let Some(loc) = &error.location() {
            log::error!("{} at {}", error, loc);
        } else {
            log::error!("{}", error);
        }
        
        error
    }
    
    // Simpler overloads for backward compatibility
    pub fn simple_request_error<T: Into<String>>(message: T) -> Self {
        Self::request_error(message, None, None::<reqwest::Error>, None)
    }
    
    pub fn simple_parse_error<T: Into<String>>(message: T) -> Self {
        Self::parse_error(message, None, None::<reqwest::Error>, None)
    }
    
    pub fn simple_domain_error<T: Into<String>>(message: T, domain: &str) -> Self {
        Self::domain_error(message, Some(domain.to_string()), None, None::<reqwest::Error>, None)
    }
    
    pub fn simple_api_error<T: Into<String>>(message: T, status: u16) -> Self {
        Self::api_error(message, Some(status), None, None)
    }
    
    // Location and source information accessors
    pub fn location(&self) -> Option<&str> {
        match self {
            Self::RequestError { location, .. } => location.as_deref(),
            Self::ParseError { location, .. } => location.as_deref(),
            Self::RateLimited { location, .. } => location.as_deref(),
            Self::MissingApiKey { location } => location.as_deref(),
            Self::ApiError { location, .. } => location.as_deref(),
            Self::ContextExceeded { location, .. } => location.as_deref(),
            Self::DomainError { location, .. } => location.as_deref(),
            _ => None,
        }
    }
    
    pub fn source_error(&self) -> Option<&(dyn std::error::Error + Send + Sync)> {
        match self {
            Self::RequestError { source, .. } => source.as_ref().map(|s| s.as_ref()),
            Self::ParseError { source, .. } => source.as_ref().map(|s| s.as_ref()),
            Self::DomainError { source, .. } => source.as_ref().map(|s| s.as_ref()),
            _ => None,
        }
    }
    
    // Legacy compat methods
    pub fn request_error_with_details<S1: Into<String>, S2: Into<String>>(
        message: S1,
        details: S2,
    ) -> Self {
        Self::request_error(message, Some(details.into()), None::<reqwest::Error>, None)
    }
    
    pub fn parse_error_with_source<S1: Into<String>, S2: Into<String>>(
        message: S1,
        source_text: S2,
    ) -> Self {
        Self::parse_error(message, Some(source_text.into()), None::<reqwest::Error>, None)
    }
    
    pub fn domain_error_with_details<S1: Into<String>, S2: Into<String>, S3: Into<String>>(
        domain: S1,
        message: S2,
        details: S3,
    ) -> Self {
        Self::domain_error(
            message, 
            Some(domain.into()), 
            Some(details.into()), 
            None::<reqwest::Error>, 
            None
        )
    }
    
    pub fn api_error_with_body<S1: Into<String>, S2: Into<String>>(
        status: u16, 
        message: S1,
        response_body: S2,
    ) -> Self {
        Self::api_error(message, Some(status), Some(response_body.into()), None)
    }
    
    // Special cases that don't fit the macro pattern
    pub fn context_exceeded() -> Self {
        Self::ContextExceeded {
            tokens: None,
            max_tokens: None,
            location: None,
        }
    }
    
    pub fn context_exceeded_with_details(tokens: u32, max_tokens: u32) -> Self {
        Self::ContextExceeded {
            tokens: Some(tokens),
            max_tokens: Some(max_tokens),
            location: None,
        }
    }
    
    pub fn rate_limited(retry_after: Option<Duration>) -> Self {
        Self::RateLimited {
            retry_after,
            details: None,
            location: None,
        }
    }
    
    pub fn rate_limited_with_details<S: Into<String>>(
        retry_after: Option<Duration>,
        details: S,
    ) -> Self {
        Self::RateLimited {
            retry_after,
            details: Some(details.into()),
            location: None,
        }
    }
}

/// Helper function to create domain-specific errors
pub fn domain_error<T>(domain: &str, message: impl Into<String>) -> ClaudeResult<T> {
    Err(ClaudeError::domain_error(message, Some(domain.to_string()), None, None::<reqwest::Error>, None))
}

/// Create a macro to capture file and line location information
#[macro_export]
macro_rules! request_error {
    ($message:expr) => {
        ClaudeError::request_error($message, None, None::<reqwest::Error>, Some(concat!(file!(), ":", line!())))
    };
    ($message:expr, $details:expr) => {
        ClaudeError::request_error($message, Some($details), None::<reqwest::Error>, Some(concat!(file!(), ":", line!())))
    };
    ($message:expr, $details:expr, $source:expr) => {
        ClaudeError::request_error($message, Some($details), Some($source), Some(concat!(file!(), ":", line!())))
    };
}

/// Create a macro for domain errors with location info
#[macro_export]
macro_rules! domain_error {
    ($domain:expr, $message:expr) => {
        ClaudeError::domain_error($message, Some($domain.to_string()), None, None::<reqwest::Error>, Some(concat!(file!(), ":", line!())))
    };
    ($domain:expr, $message:expr, $details:expr) => {
        ClaudeError::domain_error($message, Some($domain.to_string()), Some($details), None::<reqwest::Error>, Some(concat!(file!(), ":", line!())))
    };
    ($domain:expr, $message:expr, $details:expr, $source:expr) => {
        ClaudeError::domain_error($message, Some($domain.to_string()), Some($details), Some($source), Some(concat!(file!(), ":", line!())))
    };
}

/// Helper function to sanitize error messages to prevent leaking sensitive information
pub fn sanitize_error_message(message: &str) -> String {
    // Remove any potential API keys
    let api_key_pattern = regex::Regex::new(r"[A-Za-z0-9_-]{20,}").unwrap_or_else(|_| regex::Regex::new(r"").unwrap());
    let sanitized = api_key_pattern.replace_all(message, "[REDACTED]");
    
    // Could add more patterns to sanitize other sensitive information
    sanitized.into_owned()
}