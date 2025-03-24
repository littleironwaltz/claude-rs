use claude_rs::utils::*;
use claude_rs::{sanitize_error_message, ClaudeError, ClaudeResult};

// Let's implement a simple extract_json function for testing since the real one is not public
fn extract_json(text: &str) -> ClaudeResult<serde_json::Value> {
    use regex::Regex;
    use lazy_static::lazy_static;
    
    lazy_static! {
        static ref CODE_BLOCK_REGEX: Regex = Regex::new(r"```(?:json)?\s*\n([\s\S]*?)\n```").unwrap();
        static ref JSON_OBJECT_REGEX: Regex = Regex::new(r"\{[\s\S]*\}").unwrap();
    }
    
    // Try code block
    if let Some(captures) = CODE_BLOCK_REGEX.captures(text) {
        let json_str = &captures[1];
        return serde_json::from_str(json_str)
            .map_err(|e| ClaudeError::parse_error(format!("JSON error: {}", e), None, None::<reqwest::Error>, None));
    }
    
    // Try JSON object regex
    if let Some(json_match) = JSON_OBJECT_REGEX.find(text) {
        let json_str = json_match.as_str();
        return serde_json::from_str(json_str)
            .map_err(|e| ClaudeError::parse_error(format!("JSON error: {}", e), None, None::<reqwest::Error>, None));
    }
    
    Err(ClaudeError::parse_error("No JSON content found", None, None::<reqwest::Error>, None))
}

#[test]
fn test_json_extraction() {
    // Test extraction from JSON block
    let text = "Here is a result: ```json\n{\"name\": \"Claude\", \"version\": 3}\n```";
    let json = extract_json(text);
    assert!(json.is_ok());
    let json = json.unwrap();
    assert_eq!(json["name"], "Claude");
    assert_eq!(json["version"], 3);
    
    // Test extraction from non-block JSON
    let text = "The answer is {\"result\": 42}";
    let json = extract_json(text);
    assert!(json.is_ok());
    let json = json.unwrap();
    assert_eq!(json["result"], 42);
    
    // Test with no JSON present
    let text = "This text has no JSON at all";
    let json = extract_json(text);
    assert!(json.is_err());
}

#[test]
fn test_validation_functions() {
    // Test range validation
    let result = validate_range(0.5, 0.0, 1.0, "param");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0.5);
    
    // Test range validation - below minimum
    let result = validate_range(-0.1, 0.0, 1.0, "param");
    assert!(result.is_err());
    
    // Test range validation - above maximum
    let result = validate_range(1.1, 0.0, 1.0, "param");
    assert!(result.is_err());
    
    // Test string validation
    let result = StringValidator::not_empty("test", "param");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "test");
    
    // Test string validation - empty string
    let result = StringValidator::not_empty("", "param");
    assert!(result.is_err());
}

#[test]
fn test_sanitize_error_message() {
    // Use a long token that matches the regex pattern in sanitize_error_message
    // The function is looking for 20+ character alphanum strings
    let error = "Error: API key 'sk-123456789012345678901234567890' is invalid";
    let sanitized = sanitize_error_message(error);
    
    // API key should be redacted
    assert!(!sanitized.contains("sk-123456789012345678901234567890"));
    assert!(sanitized.contains("[REDACTED]"));
    
    // Test with no sensitive information
    let error = "Invalid parameter: temperature must be between 0 and 1";
    let sanitized = sanitize_error_message(error);
    assert_eq!(sanitized, error);
}