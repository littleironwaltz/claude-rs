use claude_rs::{ContextManager, Content, Role, ClaudeError};
use claude_rs::types::*;
use std::sync::Mutex;
use async_trait::async_trait;

// Simple context manager implementation for testing
struct TestContextManager {
    messages: Mutex<Vec<Message>>,
    // max_tokens is currently unused but kept for future extensions
    #[allow(dead_code)]
    max_tokens: u32,
}

impl TestContextManager {
    fn new(max_tokens: u32) -> Self {
        Self {
            messages: Mutex::new(Vec::new()),
            max_tokens,
        }
    }
    
    // Helper for testing - not part of the trait
    fn get_messages(&self) -> Vec<Message> {
        self.messages.lock().unwrap().clone()
    }
}

#[async_trait]
impl ContextManager for TestContextManager {
    async fn process_messages(&self, new_messages: Vec<Message>) -> Result<Vec<Message>, ClaudeError> {
        let messages = self.messages.lock().unwrap();
        let mut result = messages.clone();
        result.extend(new_messages);
        Ok(result)
    }
    
    async fn update_with_response(&self, response: &MessageResponse) -> Result<(), ClaudeError> {
        let mut messages = self.messages.lock().unwrap();
        
        // Extract assistant message from response and add to context
        // This assumes the first content block is what we want to use
        if let Some(Content::Text { text }) = response.content.first() {
            messages.push(Message {
                role: Role::Assistant,
                content: vec![Content::Text { text: text.clone() }],
            });
        }
        
        Ok(())
    }
}

#[tokio::test]
async fn test_context_manager() {
    let context_manager = TestContextManager::new(1000);
    
    // Test processing messages
    let new_messages = vec![Message {
        role: Role::User,
        content: vec![Content::Text { text: "Hello".to_string() }],
    }];
    
    let processed = context_manager.process_messages(new_messages).await.unwrap();
    assert_eq!(processed.len(), 1);
    
    // Test updating with response
    let response = MessageResponse {
        id: "msg_123".to_string(),
        model: "claude-3-sonnet-20240229".to_string(),
        r#type: "message".to_string(),
        role: Role::Assistant,
        content: vec![Content::Text {
            text: "Hi there".to_string()
        }],
        usage: Usage {
            input_tokens: 10,
            output_tokens: 5,
        },
        stop_reason: None,
        stop_sequence: None,
    };
    
    context_manager.update_with_response(&response).await.unwrap();
    
    // Verify context was updated
    let messages = context_manager.get_messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].role, Role::Assistant);
}