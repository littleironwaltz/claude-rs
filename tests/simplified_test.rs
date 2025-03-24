// Very simplified test that doesn't use the complicated mock infrastructure

use claude_rs::types::{MessageResponse, Role, Content, Usage};

// Helper functions to create test responses
fn create_sentiment_response(sentiment: &str, score: f64) -> MessageResponse {
    let json = format!(r#"```json
{{
    "sentiment": "{}",
    "score": {},
    "explanation": "This is a test sentiment result"
}}
```"#, sentiment, score);
    
    create_json_response(&json)
}

fn create_entity_response() -> MessageResponse {
    let json = r#"```json
    {
        "entities": [
            {
                "text": "John Smith",
                "entity_type": "Person",
                "confidence": 0.95
            },
            {
                "text": "Acme Corp",
                "entity_type": "Organization",
                "confidence": 0.88
            }
        ]
    }
    ```"#;
    
    create_json_response(json)
}

fn create_code_analysis_response() -> MessageResponse {
    let json = r#"```json
    {
        "issues": [
            {
                "line": 5,
                "severity": "High",
                "description": "Variable 'x' is undefined",
                "code": "console.log(x);"
            }
        ],
        "suggestions": [
            {
                "description": "Define variable 'x' before use",
                "original_code": "console.log(x);",
                "suggested_code": "let x = 0;\nconsole.log(x);",
                "explanation": "The variable 'x' must be defined before use"
            }
        ],
        "complexity_score": 2,
        "summary": "Found one critical issue with undefined variable"
    }
    ```"#;
    
    create_json_response(json)
}

fn create_text_response(text: &str) -> MessageResponse {
    MessageResponse {
        id: "msg_mock123".to_string(),
        model: "claude-3-sonnet-20240229".to_string(),
        r#type: "message".to_string(),
        role: Role::Assistant,
        content: vec![Content::Text { 
            text: text.to_string() 
        }],
        usage: Usage {
            input_tokens: 10,
            output_tokens: 5,
        },
        stop_reason: Some("end_turn".to_string()),
        stop_sequence: None,
    }
}

fn create_json_response(json_content: &str) -> MessageResponse {
    MessageResponse {
        id: "msg_mock123".to_string(),
        model: "claude-3-sonnet-20240229".to_string(),
        r#type: "message".to_string(),
        role: Role::Assistant,
        content: vec![Content::Text { 
            text: json_content.to_string() 
        }],
        usage: Usage {
            input_tokens: 10,
            output_tokens: 5,
        },
        stop_reason: Some("end_turn".to_string()),
        stop_sequence: None,
    }
}

// Test modules that parse the mock responses
#[cfg(test)]
mod tests {
    use super::*;
    use claude_rs::utils::json_extractor::extract_from_response;
    use serde::Deserialize;
    
    #[derive(Deserialize, Debug)]
    struct SentimentResult {
        sentiment: String,
        score: f64,
        explanation: String,
    }
    
    #[derive(Deserialize, Debug)]
    struct Entity {
        text: String,
        entity_type: String,
        #[allow(dead_code)]
        confidence: f64,
    }
    
    #[derive(Deserialize, Debug)]
    struct EntityResult {
        entities: Vec<Entity>,
    }
    
    #[derive(Deserialize, Debug)]
    struct CodeIssue {
        #[allow(dead_code)]
        line: Option<u32>,
        severity: String,
        description: String,
        #[allow(dead_code)]
        code: Option<String>,
    }
    
    #[derive(Deserialize, Debug)]
    struct CodeSuggestion {
        #[allow(dead_code)]
        description: String,
        #[allow(dead_code)]
        original_code: Option<String>,
        #[allow(dead_code)]
        suggested_code: Option<String>,
        #[allow(dead_code)]
        explanation: String,
    }
    
    #[derive(Deserialize, Debug)]
    struct CodeAnalysis {
        issues: Vec<CodeIssue>,
        suggestions: Vec<CodeSuggestion>,
        complexity_score: u32,
        #[allow(dead_code)]
        summary: String,
    }
    
    #[tokio::test]
    async fn test_sentiment_response_parsing() {
        // Create a test response
        let response = create_sentiment_response("Positive", 0.92);
        
        // Extract JSON and parse
        let json_text = extract_from_response(&response)
            .expect("Failed to extract JSON from response");
            
        let result: SentimentResult = serde_json::from_str(&json_text)
            .expect("Failed to parse JSON");
        
        // Verify results
        assert_eq!(result.sentiment, "Positive");
        assert!(result.score > 0.9);
        assert!(result.explanation.contains("test sentiment"));
    }
    
    #[tokio::test]
    async fn test_entity_response_parsing() {
        // Create a test response
        let response = create_entity_response();
        
        // Extract JSON and parse
        let json_text = extract_from_response(&response)
            .expect("Failed to extract JSON from response");
            
        let result: EntityResult = serde_json::from_str(&json_text)
            .expect("Failed to parse JSON");
        
        // Verify results
        assert_eq!(result.entities.len(), 2);
        assert_eq!(result.entities[0].text, "John Smith");
        assert_eq!(result.entities[0].entity_type, "Person");
        assert_eq!(result.entities[1].text, "Acme Corp");
    }
    
    #[tokio::test]
    async fn test_code_analysis_response_parsing() {
        // Create a test response
        let response = create_code_analysis_response();
        
        // Extract JSON and parse
        let json_text = extract_from_response(&response)
            .expect("Failed to extract JSON from response");
            
        let result: CodeAnalysis = serde_json::from_str(&json_text)
            .expect("Failed to parse JSON");
        
        // Verify results
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].description, "Variable 'x' is undefined");
        assert_eq!(result.issues[0].severity, "High");
        assert_eq!(result.suggestions.len(), 1);
        assert_eq!(result.complexity_score, 2);
    }
    
    #[tokio::test]
    async fn test_text_response_handling() {
        // Create a test response
        let response = create_text_response("This is generated content.");
        
        // Access the text directly from the response
        let text = match &response.content[0] {
            Content::Text { text } => text,
            _ => panic!("Expected text content"),
        };
        
        // Verify results
        assert_eq!(text, "This is generated content.");
    }
    
    #[tokio::test]
    async fn test_malformed_json_handling() {
        // Create a test response with malformed JSON
        let malformed_json = r#"```json
        {
            "sentiment": "Positive
            "score": 0.5,
        }
        ```"#;
        
        let response = create_json_response(malformed_json);
        
        // Extract JSON 
        let json_text = extract_from_response(&response)
            .expect("Failed to extract JSON from response");
        
        // Parsing should fail
        let result = serde_json::from_str::<SentimentResult>(&json_text);
        assert!(result.is_err());
    }
}