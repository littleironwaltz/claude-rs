//! Base implementation for domain-specific API clients.
//!
//! This module provides the foundation for domain-specific client implementations,
//! including common operations and error handling patterns.

use crate::Claude;
use std::sync::Arc;
use crate::domains::{DomainClient, DomainOperations, ValidationOperations};

/// Base client for domain-specific API implementations
/// Provides common functionality for all domain clients
pub struct BaseDomainClient {
    /// Reference to the Claude client
    claude: Arc<Claude>,
    /// Domain name for this client
    domain_name: String,
}

impl BaseDomainClient {
    /// Create a new base domain client
    pub fn new(claude: Arc<Claude>, domain_name: impl Into<String>) -> Self {
        Self { 
            claude, 
            domain_name: domain_name.into() 
        }
    }
}

impl DomainClient for BaseDomainClient {
    fn domain_name(&self) -> &str {
        &self.domain_name
    }
}

impl ValidationOperations for BaseDomainClient {}

impl DomainOperations for BaseDomainClient {
    fn claude(&self) -> &Claude {
        &self.claude
    }
}