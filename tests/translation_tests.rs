use claude_rs::{Claude, ClaudeModel};
use claude_rs::domains::translation::TranslationClient;
use std::sync::Arc;

// Import test helpers
mod test_helpers;
use test_helpers::{mock_api_to_handler, DomainTester, MockApiClient, create_json_response};

/// Helper function to create a translation domain tester
fn test_translation() -> DomainTester<TranslationClient> {
    let mock_api = Arc::new(MockApiClient::new());
    
    // Create a client that uses our mock API handler
    let client = Arc::new(Claude::with_mock_api(
        "test-key", 
        mock_api_to_handler(mock_api.clone())
    ).with_model(ClaudeModel::Sonnet));
    
    let translation_client = client.translation();
    
    DomainTester::new(translation_client, client, mock_api)
}

/// Test the translation domain client
#[tokio::test]
async fn test_translation_client() {
    // Create a translation domain tester
    let tester = test_translation();
    
    // Create a translation result JSON
    let json = r#"{
        "translated_text": "Hola mundo",
        "source_language": "English",
        "target_language": "Spanish",
        "confidence": 0.98
    }"#;
    
    // Mock the response
    tester.mock_response(ClaudeModel::Sonnet, create_json_response(json));
    
    // Test translation with max_tokens
    let result = tester.domain_client.translate_with_tokens("Hello world", "Spanish", None::<String>, Some(1000)).await.unwrap();
    
    // Verify result
    assert_eq!(result.translated_text, "Hola mundo");
    assert_eq!(result.source_language, Some("English".to_string()));
    assert_eq!(result.target_language, "Spanish");
    assert_eq!(result.confidence, Some(0.98));
    
    // Verify request contains expected text
    assert!(tester.assert_request_contains("Hello world"));
    assert!(tester.assert_request_contains("Spanish"));
}

/// Test the language detection functionality
#[tokio::test]
async fn test_language_detection() {
    // Create a translation domain tester
    let tester = test_translation();
    
    // Create a language detection result JSON
    let json = r#"{
        "language": "ja",
        "name": "Japanese",
        "confidence": 0.95
    }"#;
    
    // Mock the response
    tester.mock_response(ClaudeModel::Sonnet, create_json_response(json));
    
    // Test language detection with max_tokens
    let result = tester.domain_client.detect_language_with_tokens("こんにちは", Some(500)).await.unwrap();
    
    // Verify result
    assert_eq!(result.language, "ja");
    assert_eq!(result.name, Some("Japanese".to_string()));
    assert_eq!(result.confidence, 0.95);
    
    // Verify request contains expected text
    assert!(tester.assert_request_contains("こんにちは"));
    assert!(tester.assert_request_contains("language"));
}

/// Test alternative translations functionality
#[tokio::test]
async fn test_alternative_translations() {
    // Create a translation domain tester
    let tester = test_translation();
    
    // Create a more complex translation result JSON
    let json = r#"{
        "translated_text": "Es regnet Katzen und Hunde, aber jede Wolke hat einen Silberstreifen.",
        "source_language": "English",
        "target_language": "German",
        "confidence": 0.92,
        "alternatives": [
            {
                "original": "raining cats and dogs",
                "alternative": "es gießt wie aus Eimern",
                "context": "More idiomatic German expression for heavy rain"
            },
            {
                "original": "every cloud has a silver lining",
                "alternative": "auf Regen folgt Sonnenschein",
                "context": "German equivalent idiom"
            }
        ]
    }"#;
    
    // Mock the response
    tester.mock_response(ClaudeModel::Sonnet, create_json_response(json));
    
    // Test translation with alternatives and max_tokens
    let result = tester.domain_client.translate_with_alternatives_and_tokens(
        "It's raining cats and dogs, but every cloud has a silver lining.",
        "German",
        Some(2),
        Some(1500)
    ).await.unwrap();
    
    // Verify the main translation
    assert_eq!(result.translated_text, "Es regnet Katzen und Hunde, aber jede Wolke hat einen Silberstreifen.");
    assert_eq!(result.target_language, "German");
    
    // Verify alternatives
    let alternatives = result.alternatives.unwrap();
    assert_eq!(alternatives.len(), 2);
    
    // Check the first alternative
    assert_eq!(alternatives[0].original, "raining cats and dogs");
    assert_eq!(alternatives[0].alternative, "es gießt wie aus Eimern");
    
    // Check the second alternative
    assert_eq!(alternatives[1].original, "every cloud has a silver lining");
    assert_eq!(alternatives[1].alternative, "auf Regen folgt Sonnenschein");
    
    // Verify request contains expected text
    assert!(tester.assert_request_contains("raining cats and dogs"));
    assert!(tester.assert_request_contains("German"));
}