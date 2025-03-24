//! Domain-Specific API Implementations
//!
//! This module contains specialized clients for different domain-specific tasks.
//! Each domain client implements the `DomainClient` and `DomainOperations` traits
//! and provides more targeted functionality for specific use cases.
//!
//! ## Architecture
//!
//! The domain client system uses a trait-based approach:
//!
//! - `DomainClient` trait: Defines common validation and error handling methods
//! - `DomainOperations` trait: Defines common operations like JSON extraction
//! - `BaseDomainClient`: Implements both traits and serves as a composition base
//!
//! Domain clients use composition rather than inheritance by containing a
//! `BaseDomainClient` instance and delegating trait implementations to it.
//!
//! ## Example: Creating a Custom Domain Client
//!
//! ```rust
//! use claude_rs::{Claude, ClaudeResult};
//! use claude_rs::domains::{DomainClient, DomainOperations, base::BaseDomainClient};
//! use claude_rs::utils::StringValidator;
//! use std::sync::Arc;
//!
//! struct TranslationClient {
//!     base: BaseDomainClient
//! }
//!
//! impl TranslationClient {
//!     pub fn new(claude: Arc<Claude>) -> Self {
//!         Self {
//!             base: BaseDomainClient::new(claude, "translation")
//!         }
//!     }
//!     
//!     pub async fn translate(&self, text: impl Into<String>, language: impl Into<String>)
//!         -> ClaudeResult<String> {
//!         let text = StringValidator::not_empty(text, "text")?;
//!         let language = StringValidator::not_empty(language, "language")?;
//!         
//!         let prompt = format!("Translate to {}:\n\n{}", language, text);
//!         self.text_operation(&prompt, None, self.domain_name(), Some(1000)).await
//!     }
//! }
//!
//! impl DomainClient for TranslationClient {
//!     fn domain_name(&self) -> &str {
//!         self.base.domain_name()
//!     }
//! }
//!
//! impl DomainOperations for TranslationClient {
//!     fn claude(&self) -> &Claude {
//!         self.base.claude()
//!     }
//! }
//! ```

pub mod base;
pub mod sentiment;
pub mod entity;
pub mod content;
pub mod code;
pub mod translation;

// Re-export domain clients
pub use sentiment::{SentimentAnalysisClient, SentimentResult, Sentiment, AspectSentiment};
pub use entity::{EntityExtractionClient, Entity, EntityType};
pub use content::{ContentGenerationClient, ContentTemplate};
pub use code::{CodeAssistanceClient, CodeAnalysis, CodeIssue, CodeSuggestion, IssueSeverity};
pub use translation::{TranslationClient, TranslationResult, TranslationAlternative, DetectedLanguage};

use std::sync::{Arc, OnceLock};
use dashmap::DashMap;
use crate::Claude;
use serde::de::DeserializeOwned;
use crate::types::*;
use crate::domain_error;

/// Common trait for all domain clients
/// 
/// This trait defines the common interface that all domain clients must implement.
/// It provides domain identification for the registry system.
pub trait DomainClient: Send + Sync {
    /// The domain name for this client
    fn domain_name(&self) -> &str;
}

/// Common trait for validation operations
pub trait ValidationOperations: DomainClient {
    /// Creates a domain-specific error
    fn domain_error<T>(&self, message: impl Into<String>) -> ClaudeResult<T> {
        Err(domain_error!(
            self.domain_name(),
            message.into()
        ))
    }
    
    /// Creates a domain-specific error with details
    fn domain_error_with_details<T>(&self, message: impl Into<String>, details: impl Into<String>) -> ClaudeResult<T> {
        Err(domain_error!(
            self.domain_name(),
            message.into(),
            details.into()
        ))
    }
    
    /// Validate a string parameter
    fn validate_string<S: Into<String>>(
        &self,
        value: S,
        param_name: &str,
    ) -> ClaudeResult<String> {
        let string = value.into();
        if string.trim().is_empty() {
            return self.domain_error(format!("{} cannot be empty", param_name));
        }
        Ok(string)
    }

    /// Validate a numeric parameter is within range
    fn validate_range<T: PartialOrd + Copy + std::fmt::Debug>(
        &self,
        value: T,
        min: T,
        max: T,
        param_name: &str,
    ) -> ClaudeResult<T> {
        if value < min || value > max {
            return self.domain_error(format!(
                "{} must be between {:?} and {:?}",
                param_name, min, max
            ));
        }
        Ok(value)
    }

    /// Validate a collection is not empty
    fn validate_not_empty<C: AsRef<[T]>, T>(
        &self,
        collection: C,
        param_name: &str,
    ) -> ClaudeResult<C> {
        if collection.as_ref().is_empty() {
            return self.domain_error(format!("{} cannot be empty", param_name));
        }
        Ok(collection)
    }
}

/// Common implementation for domain operations
pub trait DomainOperations: DomainClient {
    /// Get a reference to the Claude client
    fn claude(&self) -> &Claude;
    
    /// Execute a prompt and return the raw response
    fn execute_prompt<'a>(&'a self, prompt: &'a str, temperature: Option<f32>, max_tokens: Option<u32>) -> JsonFuture<'a, MessageResponse> {
        Box::pin(async move {
            let mut builder = self.claude().message().user_message(prompt)?;
            
            if let Some(temp) = temperature {
                builder = builder.temperature(temp)?;
            }
            
            // Use max_tokens with this priority:
            // 1. Method parameter (if provided)
            // 2. Client default_max_tokens (if set)
            // 3. Fallback to 1000 as default value
            if let Some(tokens) = max_tokens.or(self.claude().default_max_tokens) {
                builder = builder.max_tokens(tokens)?;
            } else {
                // Fallback default
                builder = builder.max_tokens(1000)?;
            }
            
            builder.send().await
        })
    }
    
    /// Extract and parse JSON from a response
    fn extract_json<'a, T: DeserializeOwned>(&'a self, response: &'a MessageResponse, domain_name: &str) 
        -> JsonFuture<'a, T> {
        let domain_name = domain_name.to_string(); // Clone for async move block
        Box::pin(async move {
            let json_text = crate::utils::json_extractor::extract_from_response(response)
                .map_err(|_| domain_error!(
                    domain_name.clone(),
                    "Failed to extract JSON from response"
                ))?;
                
            serde_json::from_str(&json_text)
                .map_err(|e| domain_error!(
                    domain_name.clone(),
                    format!("Failed to parse JSON: {}", e),
                    json_text.clone()
                ))
        })
    }
    
    /// Extract text from a response
    fn extract_text(&self, response: &MessageResponse, domain_name: &str) -> ClaudeResult<String> {
        if let Some(Content::Text { text }) = response.content.first() {
            Ok(text.clone())
        } else {
            Err(domain_error!(
                domain_name,
                "No text content in response"
            ))
        }
    }
    
    /// Execute a JSON domain operation
    fn json_operation<'a, T: DeserializeOwned>(
        &'a self,
        prompt: &'a str, 
        temperature: Option<f32>,
        domain_name: &str,
        max_tokens: Option<u32>
    ) -> JsonFuture<'a, T> {
        let prompt = prompt.to_string(); // Clone for async move block
        let domain_name = domain_name.to_string(); // Clone for async move block
        Box::pin(async move {
            // Use the updated execute_prompt with max_tokens
            let response = self.execute_prompt(&prompt, temperature, max_tokens).await?;
            self.extract_json(&response, &domain_name).await
        })
    }
    
    /// Execute a text domain operation
    fn text_operation<'a>(
        &'a self,
        prompt: &'a str, 
        temperature: Option<f32>,
        domain_name: &str,
        max_tokens: Option<u32>
    ) -> TextFuture<'a> {
        let prompt = prompt.to_string(); // Clone for async move block
        let domain_name = domain_name.to_string(); // Clone for async move block
        Box::pin(async move {
            // Use the updated execute_prompt with max_tokens
            let response = self.execute_prompt(&prompt, temperature, max_tokens).await?;
            self.extract_text(&response, &domain_name)
        })
    }
}

/// Registry for domain-specific clients that provides a central access point.
/// This allows for both direct accessor methods and the domains() method approach.
pub struct DomainClientRegistry {
    claude: Arc<Claude>,
    // DashMap for lock-free concurrent access
    clients: Arc<DashMap<String, Arc<dyn DomainClient>>>,
    // Cached instances of frequently-used domain clients
    sentiment_client: OnceLock<Arc<SentimentAnalysisClient>>,
    entity_client: OnceLock<Arc<EntityExtractionClient>>,
    content_client: OnceLock<Arc<ContentGenerationClient>>,
    code_client: OnceLock<Arc<CodeAssistanceClient>>,
    translation_client: OnceLock<Arc<TranslationClient>>,
}

impl DomainClientRegistry {
    /// Create a new domain client registry associated with a Claude client
    pub(crate) fn new(claude: Arc<Claude>) -> Self {
        Self {
            claude,
            clients: Arc::new(DashMap::new()),
            sentiment_client: OnceLock::new(),
            entity_client: OnceLock::new(),
            content_client: OnceLock::new(),
            code_client: OnceLock::new(),
            translation_client: OnceLock::new(),
        }
    }

    /// Get a sentiment analysis client (optimized with caching)
    pub fn sentiment(&self) -> Arc<SentimentAnalysisClient> {
        self.sentiment_client.get_or_init(|| {
            Arc::new(SentimentAnalysisClient::new(self.claude.clone()))
        }).clone()
    }

    /// Get an entity extraction client (optimized with caching)
    pub fn entity(&self) -> Arc<EntityExtractionClient> {
        self.entity_client.get_or_init(|| {
            Arc::new(EntityExtractionClient::new(self.claude.clone()))
        }).clone()
    }

    /// Get a content generation client (optimized with caching)
    pub fn content(&self) -> Arc<ContentGenerationClient> {
        self.content_client.get_or_init(|| {
            Arc::new(ContentGenerationClient::new(self.claude.clone()))
        }).clone()
    }

    /// Get a code assistance client (optimized with caching)
    pub fn code(&self) -> Arc<CodeAssistanceClient> {
        self.code_client.get_or_init(|| {
            Arc::new(CodeAssistanceClient::new(self.claude.clone()))
        }).clone()
    }
    
    /// Get a translation client (optimized with caching)
    pub fn translation(&self) -> Arc<TranslationClient> {
        self.translation_client.get_or_init(|| {
            Arc::new(TranslationClient::new(self.claude.clone()))
        }).clone()
    }

    /// Register a custom domain client (lock-free)
    pub fn register<T: DomainClient + 'static>(&self, name: &str, client: T) {
        self.clients.insert(name.to_string(), Arc::new(client));
    }

    /// Get a registered custom domain client by name (lock-free)
    pub fn get(&self, name: &str) -> Option<Arc<dyn DomainClient>> {
        self.clients.get(name).map(|r| r.value().clone())
    }
    
    /// Get all registered domains
    pub fn list_domains(&self) -> Vec<String> {
        self.clients.iter()
            .map(|entry| entry.key().clone())
            .collect()
    }
}