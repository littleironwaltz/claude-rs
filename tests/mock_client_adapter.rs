use claude_rs::{Claude, ClaudeResult};
use claude_rs::types::*;
use claude_rs::client::MockApiHandler;
use std::sync::Arc;
use std::future::Future;
use std::pin::Pin;

// No need to import MockApiClient here

/// Extension trait to add mocking capabilities to the Claude client
pub trait MockClientAdapter {
    /// Create a new Claude client with a mock API client
    #[allow(dead_code)]
    fn with_mock_api<T>(api_key: &str, mock_api: T) -> Arc<Self>
    where
        T: Into<Arc<dyn MockApiHandler>> + Send + Sync + 'static;
    
    /// Set the mock API client for an existing Claude client
    fn set_mock_api<T>(&self, mock_api: T)
    where
        T: Into<Arc<dyn MockApiHandler>> + Send + Sync + 'static;
}

impl MockClientAdapter for Claude {
    fn with_mock_api<T>(api_key: &str, mock_api: T) -> Arc<Self>
    where
        T: Into<Arc<dyn MockApiHandler>> + Send + Sync + 'static
    {
        let client = Arc::new(Claude::new(api_key));
        client.set_mock_api(mock_api);
        client
    }
    
    fn set_mock_api<T>(&self, mock_api: T)
    where
        T: Into<Arc<dyn MockApiHandler>> + Send + Sync + 'static
    {
        // Convert to Arc<dyn MockApiHandler>
        let mock_api_handle = mock_api.into();
        let mock_api_handle_clone = mock_api_handle.clone();
        
        // Override the execute_request method with our mock implementation
        self.set_request_handler(Box::new(move |request: MessageRequest| {
            let mock = mock_api_handle.clone();
            Box::pin(async move {
                mock.process_request(request).await
            }) as Pin<Box<dyn Future<Output = ClaudeResult<MessageResponse>> + Send>>
        }));
        
        // Override the stream_request method with our mock implementation
        self.set_stream_handler(Box::new(move |request: MessageRequest| {
            let mock = mock_api_handle_clone.clone();
            Box::pin(async move {
                mock.process_stream_request(request).await
            }) as Pin<Box<dyn Future<Output = ClaudeResult<MessageStream>> + Send>>
        }));
    }
}