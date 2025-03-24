// Reactive extensions for the Claude SDK

use crate::types::*;
use crate::client::Claude;
use crate::builder::MessageBuilder;
use futures::stream::{Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};

/// Status of the reactive streaming response
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReactiveResponseStatus {
    /// The stream is initializing and no content has been received yet
    Initializing,
    /// Content is actively streaming
    Streaming,
    /// The response is complete and all content has been received
    Complete,
    /// An error occurred during streaming
    Error,
}

/// Reactive response wrapper for streaming Claude responses
pub struct ReactiveResponse {
    /// The inner stream of DeltaEvents
    pub inner: Pin<Box<dyn Stream<Item = Result<DeltaEvent, ClaudeError>> + Send>>,
    /// Accumulated text buffer
    pub text_buffer: String,
    /// Current status of the reactive response
    pub status: ReactiveResponseStatus,
    /// Last error that occurred, if any
    pub last_error: Option<ClaudeError>,
}

impl ReactiveResponse {
    /// Create a new reactive response
    pub fn new(stream: impl Stream<Item = Result<DeltaEvent, ClaudeError>> + Send + 'static) -> Self {
        Self {
            inner: Box::pin(stream),
            text_buffer: String::new(),
            status: ReactiveResponseStatus::Initializing,
            last_error: None,
        }
    }
    
    /// Get the current status of the reactive response
    pub fn status(&self) -> ReactiveResponseStatus {
        self.status
    }
    
    /// Check if streaming is complete
    pub fn is_complete(&self) -> bool {
        self.status == ReactiveResponseStatus::Complete
    }
    
    /// Get the current accumulated text
    pub fn current_text(&self) -> &str {
        &self.text_buffer
    }
    
    /// Get the last error if one occurred during streaming
    pub fn last_error(&self) -> Option<&ClaudeError> {
        self.last_error.as_ref()
    }
    
    /// Check if there was an error during streaming
    pub fn has_error(&self) -> bool {
        self.status == ReactiveResponseStatus::Error
    }
    
    /// Transform the stream to operate on text chunks
    pub fn text_stream(self) -> impl Stream<Item = Result<String, ClaudeError>> {
        let stream = self.inner;
        
        stream.map(|result| {
            match result {
                Ok(delta) => {
                    // Use the format-agnostic helper method
                    if let Some(text) = delta.to_text() {
                        return Ok(text);
                    }
                    Ok(String::new())
                }
                Err(e) => Err(e),
            }
        })
        .filter(|result| futures::future::ready(!matches!(result, Ok(s) if s.is_empty())))
    }
}

impl Stream for ReactiveResponse {
    type Item = Result<DeltaEvent, ClaudeError>;
    
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.status == ReactiveResponseStatus::Complete || 
           self.status == ReactiveResponseStatus::Error {
            return Poll::Ready(None);
        }
        
        match self.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(delta))) => {
                self.status = ReactiveResponseStatus::Streaming;
                
                // Use format-agnostic helper methods
                if let Some(text) = delta.to_text() {
                    self.text_buffer.push_str(&text);
                }
                
                if delta.is_final() {
                    self.status = ReactiveResponseStatus::Complete;
                }
                
                Poll::Ready(Some(Ok(delta)))
            }
            Poll::Ready(Some(Err(e))) => {
                self.status = ReactiveResponseStatus::Error;
                self.last_error = Some(e.clone());
                Poll::Ready(Some(Err(e)))
            }
            Poll::Ready(None) => {
                if self.status != ReactiveResponseStatus::Error {
                    self.status = ReactiveResponseStatus::Complete;
                }
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Claude {
    /// Extension method for reactive streaming
    /// 
    /// Converts a streaming request into a ReactiveResponse that tracks status
    /// and provides additional utility methods.
    pub async fn send_reactive(&self, builder: MessageBuilder) -> Result<ReactiveResponse, ClaudeError> {
        let stream = builder.stream().await?;
        Ok(ReactiveResponse::new(stream))
    }
}