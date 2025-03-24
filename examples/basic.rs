// Import the main modules using prelude for convenience
use claude_rs::prelude::*;

/// Basic usage example showing error handling with the Claude API
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Basic Claude API Usage Example");
    
    // Create client from environment variable
    let client_result = from_env();
    
    // Check if client creation failed due to missing API key
    let claude = match client_result {
        Ok(client) => client,
        Err(e) => {
            println!("Note: API key not found, using dummy client for demonstration only.");
            println!("Error: {}\n", e);
            // For demonstration purposes only, we'll continue even though the API calls will fail
            Claude::new("dummy_key_for_demo")
        }
    };
    
    // Error handling example
    println!("\nError handling demonstration:");
    
    // This will fail validation (empty prompt)
    match claude.message().user_message("") {
        Ok(_) => println!("This should not happen"),
        Err(e) => println!("Expected validation error: {}", e),
    }
    
    // This will succeed
    println!("\nSending a valid message...");
    let response = claude.message()
        .user_message("What is the capital of France?")?
        .temperature(0.5)?
        .max_tokens(1000)?
        .send()
        .await?;
        
    // Extract text from the response
    if let Some(Content::Text { text }) = response.content.first() {
        println!("Claude's response: {}", text);
    }
    
    println!("\nExample complete!");
    Ok(())
}
