//! # claude-rs: An innovative Rust SDK for Anthropic's Claude API
//! 
//! This SDK provides a comprehensive, modular interface to Anthropic's Claude API,
//! with specialized clients for different use cases like sentiment analysis,
//! entity extraction, content generation, and code assistance.
//! 
//! ## Key Features
//! 
//! - Full support for Claude API with streaming and function calling
//! - Domain-specific clients with tailored functionality
//! - Context management for optimizing token usage
//! - Middleware support for request/response processing
//! - Optional reactive extensions for advanced streaming capabilities
//! - Secure API key handling with memory zeroing
//! - TLS security configuration
//! 
//! ## Basic Usage
//! 
//! ```no_run
//! use claude_rs::{Claude, Content, from_env};
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a client from environment variable
//!     let claude = from_env()?;
//!     
//!     // Send a simple message
//!     let response = claude.message()
//!         .user_message("What is artificial intelligence?")?
//!         .send()
//!         .await?;
//!         
//!     // Extract the response text
//!     if let Some(Content::Text { text }) = response.content.first() {
//!         println!("{}", text);
//!     }
//!     
//!     Ok(())
//! }
//! ```

pub mod types;
pub mod client;
mod builder;
mod middleware;
mod context;
pub mod domains;
pub mod utils;

#[cfg(feature = "reactive")]
pub mod reactive;

// Re-export core components
pub use client::{Claude, TlsConfig, set_tls_config, MockApiHandler};
pub use types::{ClaudeError, ClaudeModel, ClaudeResult, Content, Message, MessageStream, Role, SecureApiKey, sanitize_error_message};
pub use builder::MessageBuilder;
pub use middleware::{ContextManager, RequestMiddleware, ResponseMiddleware};
pub use context::{AdaptiveContextManager, ImportanceScorer, SimpleImportanceScorer};
pub use utils::token_counter::{TokenCounter, Claude3TokenCounter, Claude2TokenCounter, SimpleTokenCounter, get_token_counter};

// Re-export domain-specific components
pub mod prelude {
    //! Convenient imports for commonly used types and functions
    pub use crate::{Claude, ClaudeError, ClaudeModel, Content, Message, Role, from_env, SecureApiKey, TlsConfig, set_tls_config};
    pub use crate::domains::{SentimentAnalysisClient, EntityExtractionClient, ContentGenerationClient, CodeAssistanceClient, TranslationClient};
    pub use crate::utils::token_counter::{TokenCounter, Claude3TokenCounter, SimpleTokenCounter, get_token_counter};
    
    // Domain-specific types
    pub use crate::domains::{
        content::ContentTemplate,
        entity::EntityType,
        sentiment::{Sentiment, SentimentResult},
        code::{CodeAnalysis, IssueSeverity},
        translation::{TranslationResult, DetectedLanguage, TranslationAlternative},
    };
}

// Public domain access
pub use domains::{
    // Base traits
    DomainClient,
    DomainOperations,
    ValidationOperations,
    
    // Domain-specific client types
    SentimentAnalysisClient,
    EntityExtractionClient, 
    ContentGenerationClient,
    CodeAssistanceClient,
    TranslationClient,
};

// Import-specific domain types
pub use domains::sentiment::{Sentiment, SentimentResult, AspectSentiment};
pub use domains::entity::{Entity, EntityType};
pub use domains::content::ContentTemplate;
pub use domains::code::{CodeAnalysis, CodeIssue, CodeSuggestion, IssueSeverity};
pub use domains::translation::{TranslationResult, TranslationAlternative, DetectedLanguage};

// Entry point functions
pub fn new_client(api_key: impl Into<String>) -> Claude {
    Claude::new(api_key)
}

pub fn from_env() -> Result<Claude, ClaudeError> {
    match std::env::var("ANTHROPIC_API_KEY") {
        Ok(key) => Ok(Claude::new(key)),
        Err(_) => Err(ClaudeError::MissingApiKey { location: None }),
    }
}