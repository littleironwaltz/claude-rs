use claude_rs::{Claude, ClaudeModel, domain_error};
use claude_rs::types::{ClaudeError, ClaudeResult};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct CustomData {
    value: String,
    number: u32,
}

// Custom error type for demonstration
#[derive(Debug, thiserror::Error)]
enum AppError {
    // The Custom variant is included for completeness but not used in this example
    #[allow(dead_code)]
    #[error("Custom error: {0}")]
    Custom(String),
    
    #[error("Data error: {0}")]
    Data(String),
}

// Function to demonstrate error source chaining
fn parse_custom_data(data: &str) -> Result<CustomData, AppError> {
    if data.is_empty() {
        return Err(AppError::Data("Empty data provided".to_string()));
    }
    
    serde_json::from_str(data)
        .map_err(|e| AppError::Data(format!("Failed to parse JSON: {}", e)))
}

// Function that uses the error handling mechanisms from claude-rs
fn process_data(data: &str) -> ClaudeResult<CustomData> {
    // Attempt to parse the data
    match parse_custom_data(data) {
        Ok(parsed) => Ok(parsed),
        Err(e) => {
            // Create a domain error with source and location
            Err(ClaudeError::domain_error(
                format!("Error processing data: {}", e),
                Some("data_processor".to_string()),
                Some("Invalid JSON format".to_string()),
                Some(e), // Source error
                Some(file!()) // Location
            ))
        }
    }
}

// Function that uses the macro to automatically capture location
fn validate_data(data: &CustomData) -> ClaudeResult<()> {
    if data.number == 0 {
        return Err(domain_error!(
            "data_processor", 
            "Invalid number value", 
            "Number cannot be zero".to_string()
        ));
    }
    
    if data.value.is_empty() {
        return Err(domain_error!(
            "data_processor", 
            "Invalid string value", 
            "Value cannot be empty".to_string()
        ));
    }
    
    Ok(())
}

// Example of using a domain client with error handling
async fn analyze_with_domain_client(text: &str) -> ClaudeResult<String> {
    // Create a client
    let client = Arc::new(Claude::new("test-key").with_model(ClaudeModel::Sonnet));
    
    // Get a domain client (sentiment in this case)
    let sentiment_client = client.sentiment();
    
    // This will fail because we're not using a real API key
    // But we'll get a proper error with location info
    let result = sentiment_client.analyze_text(text).await;
    
    // Demonstrate error handling
    match result {
        Ok(analysis) => Ok(format!("Analysis complete: {:?}", analysis.sentiment)),
        Err(e) => {
            // Log the error with location information
            eprintln!("Error at location: {:?}", e.location());
            
            // Access the source error if available
            if let Some(source) = e.source_error() {
                eprintln!("Source error: {}", source);
            }
            
            // Return a new error that includes this error as its source
            Err(domain_error!(
                "example", 
                "Failed to analyze text", 
                format!("Original error: {}", e),
                e // Include the original error as the source
            ))
        }
    }
}

#[tokio::main]
async fn main() {
    println!("Claude-rs Error Handling Example");
    println!("================================\n");
    
    // Example 1: Parse valid data
    println!("Example 1: Parsing valid data");
    let valid_data = r#"{"value": "test", "number": 42}"#;
    match process_data(valid_data) {
        Ok(data) => println!("Successfully parsed: {:?}", data),
        Err(e) => {
            println!("Error: {}", e);
            println!("Location: {:?}", e.location());
            if let Some(source) = e.source_error() {
                println!("Source: {}", source);
            }
        }
    }
    println!();
    
    // Example 2: Parse invalid data
    println!("Example 2: Parsing invalid JSON");
    let invalid_data = r#"{"value": "test", "number": invalid}"#;
    match process_data(invalid_data) {
        Ok(data) => println!("Successfully parsed: {:?}", data),
        Err(e) => {
            println!("Error: {}", e);
            println!("Location: {:?}", e.location());
            if let Some(source) = e.source_error() {
                println!("Source: {}", source);
            }
        }
    }
    println!();
    
    // Example 3: Validate invalid data
    println!("Example 3: Validating with macros");
    let invalid_value = CustomData {
        value: "".to_string(),
        number: 0,
    };
    match validate_data(&invalid_value) {
        Ok(_) => println!("Data is valid"),
        Err(e) => {
            println!("Error: {}", e);
            println!("Location: {:?}", e.location());
        }
    }
    println!();
    
    // Example 4: Using a domain client
    println!("Example 4: Using domain client");
    match analyze_with_domain_client("This is a test").await {
        Ok(result) => println!("Result: {}", result),
        Err(e) => {
            println!("Error: {}", e);
            println!("Location: {:?}", e.location());
            if let Some(source) = e.source_error() {
                println!("Source: {}", source);
            }
        }
    }
}