// Import the core modules
use claude_rs::{self, from_env};
use claude_rs::domains::content::ContentTemplate;

/// Example demonstrating parameter validation and error handling
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Validation Examples\n");
    
    // Create client from environment variable
    let claude = from_env()?;
    
    println!("1. MessageBuilder Validation\n");
    
    // Temperature validation - out of range
    match claude.message()
        .user_message("This is a test.")?
        .temperature(1.5) {
        Ok(_) => println!("This should not happen"),
        Err(e) => println!("✓ Expected temperature error: {}", e),
    }
    
    // Empty message validation
    match claude.message().user_message("") {
        Ok(_) => println!("This should not happen"),
        Err(e) => println!("✓ Expected empty message error: {}", e),
    }
    
    println!("\n2. ContentTemplate Validation\n");
    
    // Invalid template - no placeholders
    match ContentTemplate::new("This is a template with no placeholders.") {
        Ok(_) => println!("This should not happen"),
        Err(e) => println!("✓ Expected template error: {}", e),
    }
    
    // Valid template but missing parameters
    match ContentTemplate::new("Hello, {{name}}! Welcome to {{location}}.")?.render() {
        Ok(_) => println!("This should not happen"),
        Err(e) => println!("✓ Expected missing parameter error: {}", e),
    }
    
    // Unknown parameter
    match ContentTemplate::new("Hello, {{name}}!")?.with_param("unknown", "World") {
        Ok(_) => println!("This should not happen"),
        Err(e) => println!("✓ Expected unknown parameter error: {}", e),
    }
    
    // Valid template usage
    let template = ContentTemplate::new("Hello, {{name}}!")?
        .with_param("name", "World")?;
        
    let rendered = template.render()?;
    println!("✓ Rendered template: {}", rendered);
    
    println!("\n3. Domain Client Validation\n");
    
    // Word count validation in blog_post
    match claude.content().blog_post("Test", None, Some(10000)).await {
        Ok(_) => println!("This should not happen"),
        Err(e) => println!("✓ Expected word count error: {}", e),
    }
    
    // Note: We're skipping the actual API calls to avoid handling max_tokens
    // These would normally require additional parameters in the current API version
    println!("✓ Skipping product description API calls for validation demo");
    
    println!("\nAll validation tests complete!");
    Ok(())
}