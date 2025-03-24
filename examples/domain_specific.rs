// Import the main modules
use claude_rs::{self, Content, from_env};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Domain-specific clients demonstration\n");
    println!("Note: This example has been modified to show domain-specific functionality");
    println!("while ensuring the max_tokens parameter is included (required by API changes).\n");
    
    // Create client from environment variable
    let claude = from_env()?;
    
    // --- Using basic message API to demonstrate domain-specific functionality ---
    
    // 1. Sentiment Analysis
    println!("=== Sentiment Analysis ===");
    let response = claude.message()
        .user_message("Analyze the sentiment of the following text: 'I absolutely love this product! The quality is excellent.'\nRespond with a JSON object containing 'sentiment' (Positive, Neutral, or Negative) and 'score' (a number from -1 to 1).")?
        .max_tokens(1000)?
        .send()
        .await?;
    
    if let Some(content) = response.content.first() {
        match content {
            Content::Text { text } => {
                println!("Sentiment analysis result: {}\n", text);
            },
            _ => println!("Unexpected content type"),
        }
    }
    
    // 2. Entity Extraction
    println!("=== Entity Extraction ===");
    let response = claude.message()
        .user_message("Extract entities from the following text: 'Apple Inc. is based in Cupertino.'\nRespond with a JSON array of objects, each containing 'text' (the entity) and 'entity_type' (like Organization, Location, etc.)")?
        .max_tokens(1000)?
        .send()
        .await?;
    
    if let Some(content) = response.content.first() {
        match content {
            Content::Text { text } => {
                println!("Entity extraction result: {}\n", text);
            },
            _ => println!("Unexpected content type"),
        }
    }
    
    // 3. Content Generation
    println!("=== Content Generation ===");
    let response = claude.message()
        .user_message("Write a short description about machine learning.")?
        .max_tokens(1000)?
        .send()
        .await?;
    
    if let Some(content) = response.content.first() {
        match content {
            Content::Text { text } => {
                println!("Generated content:\n{}\n", text);
            },
            _ => println!("Unexpected content type"),
        }
    }
    
    // 4. Code Analysis
    println!("=== Code Analysis ===");
    let code = r#"
    fn greet(name: &str) {
        println!("Hello, {}!", name);
    }
    "#;
    
    let response = claude.message()
        .user_message(format!("Analyze the following Rust code snippet and provide a JSON result with a 'summary' field that describes what the code does:\n\n```rust\n{}\n```", code))?
        .max_tokens(1000)?
        .send()
        .await?;
        
    if let Some(content) = response.content.first() {
        match content {
            Content::Text { text } => {
                println!("Code analysis result: {}\n", text);
            },
            _ => println!("Unexpected content type"),
        }
    }
    
    println!("Example complete!");
    Ok(())
}