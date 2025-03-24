//! Integration tests for the claude-rs library
//! These tests focus on end-to-end functionality and cross-component interactions

mod streaming_integration;
mod domain_integration;
mod error_integration;

pub mod test_helpers {
    pub use crate::test_helpers::*;
}

/// Setup the test environment with any required configuration
pub fn setup_test_environment() {
    // Set up any environment variables or global configuration needed for tests
    std::env::set_var("CLAUDE_API_TEST_MODE", "true");
}

/// Initialize the test environment once
pub fn init() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        setup_test_environment();
    });
}