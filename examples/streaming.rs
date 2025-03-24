// Import the necessary modules
use claude_rs::{self, from_env};
use futures::StreamExt;
use std::io::Write;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let claude = from_env()?;
    
    println!("Generating a story using the streaming API...\n");
    
    // Create the streaming request
    let stream = claude.message()
        .user_message("Write a short story about a robot learning to paint.")?
        .max_tokens(1000)?
        .stream()
        .await?;
    
    // Pin the stream for processing
    tokio::pin!(stream);
    
    let mut story_text = String::new();
    let mut final_event_received = false;
    
    while let Some(result) = stream.next().await {
        match result {
            Ok(delta) => {
                // Use the format-agnostic helper method to extract text
                if let Some(text) = delta.to_text() {
                    print!("{}", text);
                    std::io::stdout().flush().unwrap();
                    story_text.push_str(&text);
                }
                
                // Check if this is the final event
                if delta.is_final() {
                    println!("\n(Received final event with completion marker)");
                    final_event_received = true;
                    
                    // Print usage stats if available
                    if let Some(usage) = &delta.usage {
                        println!("Input tokens: {}, Output tokens: {}", 
                            usage.input_tokens, usage.output_tokens);
                    }
                }
            }
            Err(e) => {
                // More detailed error handling
                eprintln!("Stream error: {}", e);
                
                // Check if this might be a parsing error on the final message
                if !final_event_received {
                    eprintln!("Hint: This may be due to a format change in the Claude API response.");
                    eprintln!("The core functionality should still work despite this error.");
                }
            }
        }
    }
    
    println!("\n\n--- Generation complete ---");
    
    Ok(())
}