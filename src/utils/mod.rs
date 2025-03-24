// Utility functions

pub mod json_extractor;
pub mod token_counter;

use crate::types::*;

/// Validates a value against a constraint and returns an error if it fails
pub fn validate<T, F>(
    value: T,
    constraint: F,
    error_message: impl Into<String>,
) -> ClaudeResult<T>
where
    F: FnOnce(&T) -> bool,
{
    if constraint(&value) {
        Ok(value)
    } else {
        Err(ClaudeError::ValidationError(error_message.into()))
    }
}

/// Validates a range constraint for numeric values
pub fn validate_range<T>(
    value: T,
    min: T,
    max: T,
    param_name: &str,
) -> ClaudeResult<T>
where
    T: PartialOrd + Copy + std::fmt::Display,
{
    validate(
        value,
        |&v| v >= min && v <= max,
        format!("{} must be between {} and {}, but got {}", param_name, min, max, value),
    )
}

/// Validates a string against common constraints
pub struct StringValidator;

impl StringValidator {
    /// Validates that a string is not empty
    pub fn not_empty(value: impl Into<String>, param_name: &str) -> ClaudeResult<String> {
        let value = value.into();
        validate(
            value,
            |s| !s.is_empty(),
            format!("{} cannot be empty", param_name),
        )
    }

    /// Validates that a string has a minimum length
    #[allow(dead_code)]
    pub fn min_length(
        value: impl Into<String>,
        min_length: usize,
        param_name: &str,
    ) -> ClaudeResult<String> {
        let value = value.into();
        validate(
            value,
            |s| s.len() >= min_length,
            format!("{} must be at least {} characters", param_name, min_length),
        )
    }

    /// Validates that a string has a maximum length
    #[allow(dead_code)]
    pub fn max_length(
        value: impl Into<String>,
        max_length: usize,
        param_name: &str,
    ) -> ClaudeResult<String> {
        let value = value.into();
        validate(
            value,
            |s| s.len() <= max_length,
            format!("{} must be at most {} characters", param_name, max_length),
        )
    }

    /// Validates that a string matches a regular expression pattern
    #[allow(dead_code)]
    pub fn matches_pattern(
        value: impl Into<String>,
        pattern: &str,
        param_name: &str,
    ) -> ClaudeResult<String> {
        let value = value.into();
        let regex = regex::Regex::new(pattern).map_err(|e| {
            ClaudeError::ValidationError(format!("Invalid regex pattern: {}", e))
        })?;
        validate(
            value,
            |s| regex.is_match(s),
            format!("{} must match pattern {}", param_name, pattern),
        )
    }
}

/// Validates a collection of values
#[allow(dead_code)]
pub struct CollectionValidator;

impl CollectionValidator {
    /// Validates that a collection is not empty
    #[allow(dead_code)]
    pub fn not_empty<T, C>(collection: C, param_name: &str) -> ClaudeResult<C>
    where
        C: AsRef<[T]>,
    {
        validate(
            collection,
            |c| !c.as_ref().is_empty(),
            format!("{} cannot be empty", param_name),
        )
    }

    /// Validates that a collection has a minimum size
    #[allow(dead_code)]
    pub fn min_size<T, C>(collection: C, min_size: usize, param_name: &str) -> ClaudeResult<C>
    where
        C: AsRef<[T]>,
    {
        validate(
            collection,
            |c| c.as_ref().len() >= min_size,
            format!("{} must have at least {} items", param_name, min_size),
        )
    }

    /// Validates that a collection has a maximum size
    #[allow(dead_code)]
    pub fn max_size<T, C>(collection: C, max_size: usize, param_name: &str) -> ClaudeResult<C>
    where
        C: AsRef<[T]>,
    {
        validate(
            collection,
            |c| c.as_ref().len() <= max_size,
            format!("{} must have at most {} items", param_name, max_size),
        )
    }
}

// Helper to create domain-specific errors with context
#[allow(dead_code)]
pub(crate) fn domain_error<T>(domain: &str, message: impl Into<String>) -> ClaudeResult<T> {
    Err(ClaudeError::DomainError {
        domain: domain.to_string(),
        message: message.into(),
        details: None,
        location: None,
        source: None,
    })
}