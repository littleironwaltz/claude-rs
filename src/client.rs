// Core Client Implementation

use crate::types::*;
use crate::builder::MessageBuilder;
use crate::middleware::{ContextManager, RequestMiddleware, ResponseMiddleware};
use crate::domains::*;
use reqwest::{Client as HttpClient, header};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use std::pin::Pin;
use std::future::Future;
use lazy_static::lazy_static;

// Type aliases to simplify complex types
/// Handler for a single request operation
type RequestOp<T> = dyn Future<Output = ClaudeResult<T>> + Send;

/// Result of a request operation
type RequestHandlerFuture = Pin<Box<RequestOp<MessageResponse>>>;

/// Result of a streaming operation
type StreamHandlerFuture = Pin<Box<RequestOp<MessageStream>>>;

/// Function that processes a request and returns a future
type RequestHandlerFn = dyn Fn(MessageRequest) -> RequestHandlerFuture + Send + Sync + 'static;

/// Function that processes a streaming request and returns a stream future
type StreamHandlerFn = dyn Fn(MessageRequest) -> StreamHandlerFuture + Send + Sync + 'static;

/// Trait for mocking the Claude API for testing purposes
pub trait MockApiHandler: Send + Sync {
    /// Process a request and return a response
    fn process_request(&self, request: MessageRequest) -> RequestHandlerFuture;
    
    /// Process a streaming request and return a stream of DeltaEvents
    fn process_stream_request(&self, request: MessageRequest) -> StreamHandlerFuture;
}

lazy_static! {
    static ref CLIENT_CONFIG: Mutex<TlsConfig> = Mutex::new(TlsConfig::default());
}

/// Configuration for TLS
#[derive(Clone, Debug)]
pub struct TlsConfig {
    pub min_tls_version: Option<reqwest::tls::Version>,
    pub cert_verification: bool,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            min_tls_version: Some(reqwest::tls::Version::TLS_1_2),
            cert_verification: true,
        }
    }
}

/// Set global TLS configuration for all Claude clients
pub fn set_tls_config(config: TlsConfig) {
    if let Ok(mut cfg) = CLIENT_CONFIG.lock() {
        *cfg = config;
    }
}

#[derive(Clone)]
pub struct Claude {
    pub(crate) http_client: HttpClient,
    pub(crate) api_key: SecureApiKey,
    pub base_url: String, // Made public for testing
    pub default_model: ClaudeModel, // Made public for testing
    pub default_max_tokens: Option<u32>, // Global default for max_tokens
    pub(crate) context_manager: Option<Arc<dyn ContextManager>>,
    pub(crate) request_middleware: Vec<Arc<dyn RequestMiddleware>>,
    pub(crate) response_middleware: Vec<Arc<dyn ResponseMiddleware>>,
    domain_registry: Arc<OnceLock<Arc<DomainClientRegistry>>>,
    pub(crate) request_handler: Arc<Mutex<Option<Arc<RequestHandlerFn>>>>,
    pub(crate) stream_handler: Arc<Mutex<Option<Arc<StreamHandlerFn>>>>,
}

impl Claude {
    /// Create a new Claude client with the specified API key
    pub fn new(api_key: impl Into<String>) -> Self {
        let tls_config = match CLIENT_CONFIG.lock() {
            Ok(guard) => {
                // Clone the config so we don't hold the lock
                let config = guard.clone();
                drop(guard);
                config
            },
            Err(_) => {
                // If lock is poisoned, create a new default config
                TlsConfig::default()
            }
        };
        
        Self::with_tls_config(api_key, tls_config)
    }
    
    /// Set custom request handler for this client
    /// This is useful for testing
    pub fn set_request_handler<F>(&self, handler: Box<F>) 
    where
        F: Fn(MessageRequest) -> RequestHandlerFuture + Send + Sync + 'static
    {
        if let Ok(mut guard) = self.request_handler.lock() {
            *guard = Some(Arc::new(move |req| handler(req)));
        }
    }
    
    /// Set custom stream handler for this client
    /// This is useful for testing
    pub fn set_stream_handler<F>(&self, handler: Box<F>) 
    where
        F: Fn(MessageRequest) -> StreamHandlerFuture + Send + Sync + 'static
    {
        if let Ok(mut guard) = self.stream_handler.lock() {
            *guard = Some(Arc::new(move |req| handler(req)));
        }
    }
    
    /// Create a new Claude client with a specific TLS configuration
    fn with_tls_config(api_key: impl Into<String>, tls_config: TlsConfig) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        
        let mut builder = HttpClient::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(300))
            .danger_accept_invalid_certs(!tls_config.cert_verification);
            
        if let Some(version) = tls_config.min_tls_version {
            builder = builder.min_tls_version(version);
        }
        
        let http_client = builder.build()
            .expect("Failed to create HTTP client");
            
        Self {
            http_client,
            api_key: SecureApiKey::new(api_key),
            base_url: "https://api.anthropic.com/v1".to_string(),
            default_model: ClaudeModel::Sonnet37,
            default_max_tokens: None, // No default max_tokens initially
            context_manager: None,
            request_middleware: Vec::new(),
            response_middleware: Vec::new(),
            domain_registry: Arc::new(OnceLock::new()),
            request_handler: Arc::new(Mutex::new(None)),
            stream_handler: Arc::new(Mutex::new(None)),
        }
    }

    /// Set a default model to use for requests
    pub fn with_model(mut self, model: ClaudeModel) -> Self {
        self.default_model = model;
        self
    }
    
    /// Set a custom base URL for the API
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
    
    /// Set a default max_tokens value for all requests
    pub fn with_default_max_tokens(mut self, max_tokens: u32) -> ClaudeResult<Self> {
        if max_tokens == 0 {
            return Err(ClaudeError::ValidationError("max_tokens must be greater than 0".into()));
        }
        self.default_max_tokens = Some(max_tokens);
        Ok(self)
    }
    
    /// Add a context manager for handling message history
    pub fn with_context_manager(mut self, manager: impl ContextManager + 'static) -> Self {
        self.context_manager = Some(Arc::new(manager));
        self
    }
    
    /// Add middleware that processes requests before they're sent
    pub fn add_request_middleware(mut self, middleware: impl RequestMiddleware + 'static) -> Self {
        self.request_middleware.push(Arc::new(middleware));
        self
    }
    
    /// Add middleware that processes responses after they're received
    pub fn add_response_middleware(mut self, middleware: impl ResponseMiddleware + 'static) -> Self {
        self.response_middleware.push(Arc::new(middleware));
        self
    }
    
    /// Create a message builder for constructing a request
    pub fn message(&self) -> MessageBuilder {
        MessageBuilder::from_client(Arc::new(self.clone()))
    }
    
    /// Get the domain client registry
    pub fn domains(&self) -> Arc<DomainClientRegistry> {
        self.domain_registry.get_or_init(|| {
            let claude_arc = Arc::new(self.clone());
            Arc::new(DomainClientRegistry::new(claude_arc))
        }).clone()
    }
    
    /// Get a domain-specific client for sentiment analysis
    pub fn sentiment(&self) -> Arc<SentimentAnalysisClient> {
        self.domains().sentiment()
    }
    
    /// Get a domain-specific client for entity extraction
    pub fn entity(&self) -> Arc<EntityExtractionClient> {
        self.domains().entity()
    }
    
    /// Get a domain-specific client for content generation
    pub fn content(&self) -> Arc<ContentGenerationClient> {
        self.domains().content()
    }
    
    /// Get a domain-specific client for code assistance
    pub fn code(&self) -> Arc<CodeAssistanceClient> {
        self.domains().code()
    }
    
    /// Get a domain-specific client for translation
    pub fn translation(&self) -> Arc<TranslationClient> {
        self.domains().translation()
    }
    
    /// Register a custom domain client
    pub fn register_domain<T: DomainClient + 'static>(&self, name: &str, client: T) -> &Self {
        self.domains().register(name, client);
        self
    }
    
    /// Get a custom domain client by name
    pub fn get_domain(&self, name: &str) -> Option<Arc<dyn DomainClient>> {
        self.domains().get(name)
    }
    
    /// Create a new Claude client with a mock API for testing
    /// This is a helper method for testing purposes
    pub fn with_mock_api<T>(api_key: impl Into<String>, mock_api: T) -> Self 
    where
        T: Into<Arc<dyn MockApiHandler>> + Send + Sync + 'static
    {
        let client = Self::new(api_key);
        
        // Convert to Arc<dyn MockApiHandler>
        let mock_handler = mock_api.into();
        let mock_handler_clone = mock_handler.clone();
        
        // Set up the request handler
        client.set_request_handler(Box::new(move |request: MessageRequest| {
            let mock = mock_handler.clone();
            mock.process_request(request)
        }));
        
        // Set up the stream handler with the cloned handler
        client.set_stream_handler(Box::new(move |request: MessageRequest| {
            let mock = mock_handler_clone.clone();
            mock.process_stream_request(request)
        }));
        
        client
    }
    
    // Deprecated methods with aliases to new names
    
    /// Get a domain-specific client for sentiment analysis
    #[deprecated(since="0.2.0", note="Use sentiment() instead")]
    pub fn sentiment_analysis(&self) -> Arc<SentimentAnalysisClient> {
        self.sentiment()
    }
    
    /// Get a domain-specific client for entity extraction
    #[deprecated(since="0.2.0", note="Use entity() instead")]
    pub fn entity_extraction(&self) -> Arc<EntityExtractionClient> {
        self.entity()
    }
    
    /// Get a domain-specific client for content generation
    #[deprecated(since="0.2.0", note="Use content() instead")]
    pub fn content_generation(&self) -> Arc<ContentGenerationClient> {
        self.content()
    }
    
    /// Get a domain-specific client for code assistance
    #[deprecated(since="0.2.0", note="Use code() instead")]
    pub fn code_assistance(&self) -> Arc<CodeAssistanceClient> {
        self.code()
    }
}