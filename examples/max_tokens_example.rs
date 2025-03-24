use claude_rs::Claude;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Claude Max Tokens Example\n");
    
    // Create a client with default max_tokens setting
    let claude = Claude::new("YOUR_API_KEY") // Replace with your API key
        .with_default_max_tokens(1200)?; // Set a global default
        
    println!("Client configured with default max_tokens of 1200\n");
    
    // Get the translation client
    let _translator = claude.translation();
    
    // The global default max_tokens is used automatically (no need to specify explicitly)
    println!("1. Using global default max_tokens (1200):");
    println!("   translator.translate(text, \"Spanish\", None::<String>)");
    
    // Example of overriding with domain-specific method
    println!("\n2. Overriding with domain-specific method (800 tokens):");
    println!("   translator.translate_with_tokens(text, \"Spanish\", None::<String>, Some(800))");
    
    // Example of direct message with global default
    println!("\n3. Direct message using global default (1200):");
    println!("   claude.message().user_content(\"Translate to French: Hello world\").send()");
    
    // Example of direct message overriding default
    println!("\n4. Direct message overriding global default (500):");
    println!("   claude.message().user_content(\"Translate to German: Hello world\").max_tokens(500)?.send()");

    // Example showing operation-specific defaults still work when needed
    println!("\n5. Domain operation with specific tokens (overrides global default):");
    println!("   Entity detection (500 tokens): entity.detect_entities(text)");
    println!("   Standard translation (1000 tokens): translator.translate(text, \"Spanish\", None::<String>)");
    println!("   Complex translation alternatives (1500 tokens): translator.translate_with_alternatives(text, \"Spanish\", Some(3))");
    
    println!("\nBenefits of global default max_tokens:");
    println!("- Consistent token allocation across all operations");
    println!("- Central management of token usage");
    println!("- Can still be overridden when needed");
    println!("- Simplifies API calls by removing the need for explicit max_tokens in common cases");
    
    Ok(())
}