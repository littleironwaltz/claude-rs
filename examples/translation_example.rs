use claude_rs::from_env;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create a client from environment variable
    let claude = from_env()?;
    
    // Get the translation client (prefixed with _ since we're not making real API calls)
    let _translator = claude.translation();
    
    println!("Claude Translation Example\n");
    
    // 1. Basic translation example
    let text_to_translate = "Hello, world! This is a test of the translation domain.";
    println!("Original text: {}", text_to_translate);
    
    // In a real scenario, max_tokens is required
    // Example of how it would be used: 
    // claude.message().user_content("text").max_tokens(500)?;
    
    println!("\nNote: This example will not make real API calls without a valid API key.");
    println!("Here's how you would use the translation client with max_tokens:");
    println!("let result = translator.translate_with_tokens(text_to_translate, \"Spanish\", None::<String>, Some(1000)).await?;");
    println!("// The max_tokens parameter ensures proper response length");
    println!("\nExample translated text: \"Hola, mundo! Esto es una prueba del dominio de traducción.\"");
    
    // 2. Translation with source language specified
    let french_text = "Bonjour le monde! Comment ça va?";
    println!("\nFrench text: {}", french_text);
    
    println!("How to specify source language and max_tokens:");
    println!("let result = translator.translate_with_tokens(french_text, \"Japanese\", Some(\"French\"), Some(1000)).await?;");
    println!("Example Japanese translation: \"こんにちは、世界！元気ですか？\"");
    
    // 3. Language detection
    let mystery_text = "こんにちは世界、元気ですか？";
    println!("\nMystery text: {}", mystery_text);
    
    println!("How to detect language with max_tokens:");
    println!("let detected = translator.detect_language_with_tokens(mystery_text, Some(500)).await?;");
    println!("// Language detection typically requires fewer tokens");
    println!("Example detection result: Japanese (ja), Confidence: 0.98");
    
    // 4. Translation with alternatives
    let text_with_idioms = "It's raining cats and dogs, but every cloud has a silver lining.";
    println!("\nText with idioms: {}", text_with_idioms);
    
    println!("How to get alternative translations for idioms with max_tokens:");
    println!("let result = translator.translate_with_alternatives_and_tokens(text_with_idioms, \"German\", Some(2), Some(1500)).await?;");
    println!("// Alternative translations need more tokens for the additional content");
    println!("Example German translation: \"Es regnet Katzen und Hunde, aber jede Wolke hat einen Silberstreifen.\"");
    
    println!("\nExample alternative translations:");
    println!("- Original: \"raining cats and dogs\"");
    println!("  Alternative: \"es gießt wie aus Eimern\"");
    println!("  Context: More idiomatic German expression for heavy rain");
    println!();
    println!("- Original: \"every cloud has a silver lining\"");
    println!("  Alternative: \"auf Regen folgt Sonnenschein\"");
    println!("  Context: German equivalent idiom");
    println!();
    
    Ok(())
}