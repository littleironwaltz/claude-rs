// Context Management Implementation

use crate::types::*;
use crate::middleware::ContextManager;
use crate::utils::token_counter::{TokenCounter, Claude3TokenCounter, get_token_counter};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

/// # Adaptive Context Manager
/// 
/// The `AdaptiveContextManager` helps optimize token usage by intelligently managing
/// conversation history. It works by:
/// 
/// 1. Scoring messages by importance
/// 2. Prioritizing high-importance messages when the context exceeds the token limit
/// 3. Always including the most recent message
/// 
/// ## Example
/// 
/// ```no_run
/// # use claude_rs::{Claude, AdaptiveContextManager, SimpleImportanceScorer};
/// # let api_key = "your_api_key_here";
/// let claude = Claude::new(api_key)
///     .with_context_manager(AdaptiveContextManager::new(
///         4000, // max tokens
///         SimpleImportanceScorer
///     ));
/// ```
/// 
/// This context manager helps avoid hitting context window limits by:
/// - Tracking message history automatically
/// - Estimating token usage
/// - Dropping less important messages when needed
/// - Ensuring the most recent messages are always included
pub struct AdaptiveContextManager {
    /// Maximum number of tokens allowed in the context window
    max_tokens: u32,
    /// Scorer that determines message importance (0.0 to 1.0)
    importance_scorer: Arc<dyn ImportanceScorer>,
    /// Message history stored between requests
    history: Mutex<Vec<Message>>,
    /// Token counter for accurate token counting
    token_counter: Arc<dyn TokenCounter>,
    /// Default model for token counting when not specified
    #[allow(dead_code)]
    default_model: ClaudeModel,
}

impl AdaptiveContextManager {
    /// Create a new AdaptiveContextManager with a token limit and importance scorer
    /// 
    /// # Arguments
    /// 
    /// * `max_tokens` - Maximum number of tokens to use for context
    /// * `importance_scorer` - Implementation of ImportanceScorer that ranks message importance
    pub fn new(max_tokens: u32, importance_scorer: impl ImportanceScorer + 'static) -> Self {
        Self {
            max_tokens,
            importance_scorer: Arc::new(importance_scorer),
            history: Mutex::new(Vec::new()),
            token_counter: Arc::new(Claude3TokenCounter),
            default_model: ClaudeModel::Sonnet37,
        }
    }
    
    /// Create a new AdaptiveContextManager with a specified model and token counter
    /// 
    /// # Arguments
    /// 
    /// * `max_tokens` - Maximum number of tokens to use for context
    /// * `importance_scorer` - Implementation of ImportanceScorer that ranks message importance
    /// * `model` - The Claude model to use for token counting
    pub fn with_model(max_tokens: u32, importance_scorer: impl ImportanceScorer + 'static, model: ClaudeModel) -> Self {
        Self {
            max_tokens,
            importance_scorer: Arc::new(importance_scorer),
            history: Mutex::new(Vec::new()),
            token_counter: get_token_counter(&model),
            default_model: model,
        }
    }
    
    /// Count tokens in a text using the configured TokenCounter
    /// 
    /// This method uses the tiktoken library for accurate token counting
    /// that matches Claude's actual tokenization.
    #[allow(dead_code)]
    fn count_tokens(&self, text: &str) -> u32 {
        self.token_counter.count_tokens(text)
    }
    
    /// Count tokens in a message using the configured TokenCounter
    fn count_message_tokens(&self, message: &Message) -> u32 {
        self.token_counter.count_message_tokens(message)
    }
    
    /// Legacy token counting for backward compatibility
    #[deprecated(since = "0.2.0", note = "Use count_tokens instead")]
    #[allow(dead_code)]
    fn estimate_tokens(text: &str) -> u32 {
        // Approximate tokens as 4 characters per token
        (text.len() as u32 + 3) / 4
    }
    
    /// Clear the message history
    pub async fn clear_history(&self) {
        let mut history = self.history.lock().await;
        history.clear();
    }
    
    /// Get the current number of messages in history
    pub async fn history_size(&self) -> usize {
        let history = self.history.lock().await;
        history.len()
    }
}

#[async_trait]
pub trait ImportanceScorer: Send + Sync {
    /// Score the importance of a message (0.0 to 1.0)
    async fn score_importance(&self, message: &Message) -> f32;
}

#[async_trait]
impl ContextManager for AdaptiveContextManager {
    async fn process_messages(&self, mut messages: Vec<Message>) -> Result<Vec<Message>, ClaudeError> {
        let history = self.history.lock().await;
        
        // Add historical context
        let mut all_messages = history.clone();
        all_messages.extend(messages.clone());
        
        // Calculate total tokens with accurate token counter
        let total_tokens: u32 = all_messages.iter()
            .map(|msg| self.count_message_tokens(msg))
            .sum();
        
        // If within limit, use all messages
        if total_tokens <= self.max_tokens {
            return Ok(all_messages);
        }
        
        // Otherwise, prioritize messages based on importance
        let mut scored_messages = Vec::with_capacity(all_messages.len());
        for msg in all_messages {
            let score = self.importance_scorer.score_importance(&msg).await;
            let tokens = self.count_message_tokens(&msg);
            scored_messages.push((msg, score, tokens));
        }
        
        // Sort by importance (descending)
        scored_messages.sort_by(|(_, a, _), (_, b, _)| b.partial_cmp(a).unwrap());
        
        // Take messages until we hit the token limit
        let mut result = Vec::new();
        let mut current_tokens = 0;
        
        // Always include the most recent message
        if !messages.is_empty() {
            let latest = messages.pop().unwrap();
            let latest_tokens = self.count_message_tokens(&latest);
            current_tokens += latest_tokens;
            result.push(latest);
        }
        
        // Add other messages based on importance
        for (msg, _, tokens) in scored_messages {
            // We already have the token count for each message
            if current_tokens + tokens <= self.max_tokens {
                current_tokens += tokens;
                result.push(msg);
            } else {
                // If we can't fit the entire message, we could summarize it
                // (Implementation would go here)
            }
        }
        
        Ok(result)
    }
    
    async fn update_with_response(&self, response: &MessageResponse) -> Result<(), ClaudeError> {
        let mut history = self.history.lock().await;
        
        // Convert response to a message and add to history
        let message = Message {
            role: Role::Assistant,
            content: response.content.clone(),
        };
        
        history.push(message);
        
        Ok(())
    }
}

// Simple importance scorer implementation
pub struct SimpleImportanceScorer;

#[async_trait]
impl ImportanceScorer for SimpleImportanceScorer {
    async fn score_importance(&self, message: &Message) -> f32 {
        // In a real implementation, this would use heuristics or ML
        // For now, a simple implementation that gives higher scores to:
        // - More recent messages (if we had timestamps)
        // - User messages (assumed to be more important than assistant responses)
        // - Messages with specific keywords
        
        let base_score = match message.role {
            Role::User => 0.8,
            Role::Assistant => 0.5,
        };
        
        // Check for important keywords
        let mut keyword_bonus = 0.0;
        for content in &message.content {
            if let Content::Text { text } = content {
                if text.contains("important") || text.contains("critical") || text.contains("essential") {
                    keyword_bonus += 0.2;
                }
                
                // This is simplistic - a real implementation would be more sophisticated
            }
        }
        
        f32::min(base_score + keyword_bonus, 1.0)
    }
}