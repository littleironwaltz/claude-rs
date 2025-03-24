// Middleware and Extension Traits

use crate::types::*;
use async_trait::async_trait;

#[async_trait]
pub trait ContextManager: Send + Sync {
    /// Process and possibly modify the messages before sending
    async fn process_messages(&self, messages: Vec<Message>) -> Result<Vec<Message>, ClaudeError>;
    
    /// Update internal state based on the response
    async fn update_with_response(&self, response: &MessageResponse) -> Result<(), ClaudeError>;
}

#[async_trait]
pub trait RequestMiddleware: Send + Sync {
    /// Process and possibly modify the request before sending
    async fn process_request(&self, request: MessageRequest) -> Result<MessageRequest, ClaudeError>;
}

#[async_trait]
pub trait ResponseMiddleware: Send + Sync {
    /// Process and possibly modify the response after receiving
    async fn process_response(&self, response: MessageResponse) -> Result<MessageResponse, ClaudeError>;
}