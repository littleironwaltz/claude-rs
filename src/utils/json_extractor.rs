//! JSON extraction utilities
//!
//! This module provides robust extraction of JSON from text responses.
//! It implements multiple strategies to handle different ways Claude might
//! format JSON in its responses, from code blocks to inline objects.
//!
//! The extraction process follows this sequential approach:
//! 1. Try to extract from ```json code blocks
//! 2. Try to extract JSON objects using regex pattern matching
//! 3. Try to use the raw text if it appears to be valid JSON
//!
//! This modular approach follows the Open/Closed principle, making it
//! easy to add new extraction strategies in the future.

use crate::types::*;
use regex::Regex;
use lazy_static::lazy_static;

// Pre-compile regular expressions for better performance
lazy_static! {
    static ref CODE_BLOCK_REGEX: Regex = Regex::new(r"```(?:json)?\s*\n([\s\S]*?)\n```").unwrap();
    static ref JSON_OBJECT_REGEX: Regex = Regex::new(r"\{[\s\S]*\}").unwrap();
    static ref JSON_ARRAY_REGEX: Regex = Regex::new(r"\[[\s\S]*\]").unwrap();
}

/// Extract JSON from a Claude message response
/// 
/// This function tries multiple extraction strategies in sequence, from most
/// reliable to least reliable. It returns the first successful extraction.
///
/// # Arguments
///
/// * `response` - The MessageResponse from Claude API
///
/// # Returns
///
/// * `Ok(String)` - The extracted JSON string
/// * `Err(ClaudeError)` - If no JSON could be extracted using any strategy
///
/// # Example
///
/// ```ignore
/// // This is a simplified example of how you might use this function
/// use serde::Deserialize;
/// 
/// #[derive(Deserialize)]
/// struct MyData { value: String }
/// 
/// // After getting a response from Claude
/// let json_str = extract_from_response(&response)?;
/// let data: MyData = serde_json::from_str(&json_str)
///     .map_err(|e| ClaudeError::ParseError(format!("JSON error: {}", e)))?;
/// ```
pub fn extract_from_response(response: &MessageResponse) -> Result<String, ClaudeError> {
    // Try each strategy in sequence
    extract_from_code_block(response)
        .or_else(|_| extract_from_json_object(response))
        .or_else(|_| extract_raw_text(response))
}

/// Extract JSON from a code block with ```json markers
///
/// This is the preferred extraction method since code blocks typically
/// contain well-formatted JSON. Claude often returns JSON in this format
/// when explicitly asked to respond with JSON.
///
/// # Strategy
/// 
/// Looks for content wrapped in code block markers (```json ... ```)
/// and extracts the content inside.
fn extract_from_code_block(response: &MessageResponse) -> Result<String, ClaudeError> {
    for content in &response.content {
        if let Content::Text { text } = content {
            if let Some(captures) = CODE_BLOCK_REGEX.captures(text) {
                return Ok(captures[1].to_string());
            }
        }
    }
    Err(ClaudeError::parse_error("No JSON code block found in response", None, None::<serde_json::Error>, None))
}

/// Extract JSON from object notation { ... } or array [ ... ]
///
/// This is the fallback method when code blocks aren't found. It looks for
/// JSON-like structures and extracts them directly from the text.
///
/// # Strategy
/// 
/// 1. First tries to find patterns that match JSON objects: {...}
/// 2. If not found, tries to find patterns that match JSON arrays: [...]
fn extract_from_json_object(response: &MessageResponse) -> Result<String, ClaudeError> {
    for content in &response.content {
        if let Content::Text { text } = content {
            // Try to find JSON objects first (more common)
            if let Some(json_match) = JSON_OBJECT_REGEX.find(text) {
                return Ok(json_match.as_str().to_string());
            }
            
            // Then try arrays if no objects found
            if let Some(json_match) = JSON_ARRAY_REGEX.find(text) {
                return Ok(json_match.as_str().to_string());
            }
        }
    }
    Err(ClaudeError::parse_error("No JSON object or array found in response", None, None::<serde_json::Error>, None))
}

/// Treat the entire text as JSON if it appears to be valid
/// 
/// This is the last resort method that tries to use the raw text as JSON
/// if it appears to be valid JSON (starts with { or [).
///
/// # Strategy
///
/// Checks if the trimmed text starts with { or [ and returns it directly.
/// This is useful when Claude returns clean JSON without any preamble or code blocks.
fn extract_raw_text(response: &MessageResponse) -> Result<String, ClaudeError> {
    for content in &response.content {
        if let Content::Text { text } = content {
            let trimmed = text.trim();
            // Simple heuristic to check if the entire content might be JSON
            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                return Ok(trimmed.to_string()); // Return trimmed text to avoid whitespace issues
            }
        }
    }
    Err(ClaudeError::parse_error("No JSON content found in response", None, None::<serde_json::Error>, None))
}